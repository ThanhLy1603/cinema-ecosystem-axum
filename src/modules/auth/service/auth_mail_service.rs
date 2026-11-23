use crate::infrastructure::mailer::{Mailer, OutgoingMail};

const PASSWORD_RESET_PATH: &str = "/reset-password";
const EMAIL_VERIFICATION_PATH: &str = "/verify-email";

pub fn build_link(frontend_base_url: &str, path: &str, raw_token: &str) -> String {
   format!("{}{}?token={}", frontend_base_url.trim_end_matches('/'), path, raw_token)
}

pub fn send_password_reset(mailer: &Mailer, frontend_base_url: &str, email: &str, full_name: &str, raw_token: &str) {
   let link: String = build_link(frontend_base_url, PASSWORD_RESET_PATH, raw_token);
   let body: String = format!(
      "Xin chào {full_name},\n\n\
       Chúng tôi nhận được yêu cầu đặt lại mật khẩu cho tài khoản CinemaSphere của bạn.\n\
       Bấm vào liên kết dưới đây để đặt mật khẩu mới:\n\n{link}\n\n\
       Liên kết có hiệu lực trong 30 phút và chỉ dùng được một lần. \
       Nếu bạn không yêu cầu, hãy bỏ qua email này, mật khẩu của bạn vẫn được giữ nguyên.\n\n\
       CinemaSphere"
   );
   mailer.send(OutgoingMail {
      to: email.to_string(), subject: "[CinemaSphere] Đặt lại mật khẩu".to_string(), body
   });
}

pub fn send_email_verification(
   mailer: &Mailer,
   frontend_base_url: &str,
   email: &str,
   full_name: &str,
   raw_token: &str,
) {
   let link: String = build_link(frontend_base_url, EMAIL_VERIFICATION_PATH, raw_token);
   let body: String = format!(
      "Xin chào {full_name},\n\n\
       Cảm ơn bạn đã đăng ký CinemaSphere. Bấm vào liên kết dưới đây để xác thực email:\n\n{link}\n\n\
       Liên kết có hiệu lực trong 24 giờ. Nếu bạn không đăng ký tài khoản này, hãy bỏ qua email.\n\n\
       CinemaSphere"
   );
   mailer.send(OutgoingMail {
      to: email.to_string(),
      subject: "[CinemaSphere] Xác thực email của bạn".to_string(),
      body,
   });
}

#[cfg(test)]
mod tests {
   use super::{build_link, send_email_verification, send_password_reset};
   use crate::infrastructure::mailer::Mailer;

   #[test]
   fn link_has_no_double_slash_and_carries_token() {
      assert_eq!(
         build_link("http://localhost:5173/", "/reset-password", "abc"),
         "http://localhost:5173/reset-password?token=abc"
      );
   }

   #[test]
   fn templates_are_vietnamese_and_contain_the_link() {
      let (mailer, outbox) = Mailer::recording();
      send_password_reset(&mailer, "http://localhost:5173", "a@example.com", "Nguyễn An", "tok1");
      send_email_verification(&mailer, "http://localhost:5173", "b@example.com", "Trần Bình", "tok2");

      let recorded_mails = outbox.lock().unwrap();
      assert_eq!(recorded_mails.len(), 2);
      assert_eq!(recorded_mails[0].subject, "[CinemaSphere] Đặt lại mật khẩu");
      assert!(recorded_mails[0].body.contains("Xin chào Nguyễn An"));
      assert!(recorded_mails[0].body.contains("http://localhost:5173/reset-password?token=tok1"));
      assert_eq!(recorded_mails[1].subject, "[CinemaSphere] Xác thực email của bạn");
      assert!(recorded_mails[1].body.contains("http://localhost:5173/verify-email?token=tok2"));
   }
}
