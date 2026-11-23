use std::sync::{Arc, Mutex};
use std::time::Duration;

use lettre::message::header::ContentType;
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

use crate::config::MailConfig;

const SMTP_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Debug)]
pub struct OutgoingMail {
   pub to: String,
   pub subject: String,
   pub body: String,
}

enum MailDelivery {
   Smtp { transport: Box<AsyncSmtpTransport<Tokio1Executor>>, from: Mailbox },
   Recording(Arc<Mutex<Vec<OutgoingMail>>>),
}

pub struct Mailer {
   delivery: MailDelivery,
}

impl Mailer {
   pub fn smtp(mail_config: &MailConfig) -> Result<Mailer, String> {
      let from: Mailbox =
         mail_config.from.parse().map_err(|_| format!("MAIL_FROM không hợp lệ: {}", mail_config.from))?;
      let transport_builder = if mail_config.starttls {
         AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&mail_config.host)
            .map_err(|smtp_error| format!("MAIL_HOST không hợp lệ: {smtp_error}"))?
      } else {
         AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&mail_config.host)
      };
      let transport_builder = transport_builder.port(mail_config.port).timeout(Some(SMTP_TIMEOUT));
      let transport_builder = if mail_config.username.is_empty() {
         transport_builder
      } else {
         transport_builder.credentials(Credentials::new(mail_config.username.clone(), mail_config.password.clone()))
      };
      Ok(Mailer { delivery: MailDelivery::Smtp { transport: Box::new(transport_builder.build()), from } })
   }

   pub fn recording() -> (Mailer, Arc<Mutex<Vec<OutgoingMail>>>) {
      let outbox: Arc<Mutex<Vec<OutgoingMail>>> = Arc::new(Mutex::new(Vec::new()));
      (Mailer { delivery: MailDelivery::Recording(outbox.clone()) }, outbox)
   }

   pub fn send(&self, outgoing_mail: OutgoingMail) {
      match &self.delivery {
         MailDelivery::Recording(outbox) => {
            if let Ok(mut recorded_mails) = outbox.lock() {
               recorded_mails.push(outgoing_mail);
            }
         }
         MailDelivery::Smtp { transport, from } => {
            let transport: AsyncSmtpTransport<Tokio1Executor> = (**transport).clone();
            let from: Mailbox = from.clone();
            tokio::spawn(async move {
               if let Err(reason) = deliver(&transport, from, &outgoing_mail).await {
                  tracing::warn!(
                     subject = %outgoing_mail.subject,
                     to = %mask_email(&outgoing_mail.to),
                     %reason,
                     "Không gửi được email"
                  );
               }
            });
         }
      }
   }
}

async fn deliver(
   transport: &AsyncSmtpTransport<Tokio1Executor>,
   from: Mailbox,
   outgoing_mail: &OutgoingMail,
) -> Result<(), String> {
   let to: Mailbox = outgoing_mail.to.parse().map_err(|_| "địa chỉ người nhận không hợp lệ".to_string())?;
   let message: Message = Message::builder()
      .from(from)
      .to(to)
      .subject(outgoing_mail.subject.clone())
      .header(ContentType::TEXT_PLAIN)
      .body(outgoing_mail.body.clone())
      .map_err(|message_error| message_error.to_string())?;
   transport.send(message).await.map(|_| ()).map_err(|smtp_error| smtp_error.to_string())
}

pub fn mask_email(email: &str) -> String {
   let Some(at_index) = email.find('@') else {
      return "***".to_string();
   };
   let domain_part: &str = &email[at_index..];
   match email[..at_index].chars().next() {
      Some(first_character) if email[..at_index].chars().count() > 1 => format!("{first_character}***{domain_part}"),
      _ => format!("***{domain_part}"),
   }
}

#[cfg(test)]
mod tests {
   use super::{mask_email, Mailer, OutgoingMail};

   #[test]
   fn recording_mailer_keeps_what_was_sent() {
      let (mailer, outbox) = Mailer::recording();
      mailer.send(OutgoingMail {
         to: "a@example.com".to_string(),
         subject: "Chủ đề".to_string(),
         body: "Nội dung".to_string(),
      });
      let recorded_mails = outbox.lock().unwrap();
      assert_eq!(recorded_mails.len(), 1);
      assert_eq!(recorded_mails[0].to, "a@example.com");
      assert_eq!(recorded_mails[0].subject, "Chủ đề");
   }

   #[test]
   fn masks_local_part_of_emails() {
      assert_eq!(mask_email("nguyen@example.com"), "n***@example.com");
      assert_eq!(mask_email("a@example.com"), "***@example.com");
      assert_eq!(mask_email("khong-co-a-cong"), "***");
   }
}
