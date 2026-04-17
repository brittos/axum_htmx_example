use askama::Template;

#[derive(Template)]
#[template(path = "auth/forgot_password.html")]
pub struct ForgotPasswordTemplate {
    pub error_message: Option<String>,
    pub success_message: Option<String>,
    pub csrf_token: String,
}

#[derive(Template)]
#[template(path = "auth/reset_password.html")]
pub struct ResetPasswordTemplate {
    pub token: String,
    pub error_message: Option<String>,
    pub csrf_token: String,
}
