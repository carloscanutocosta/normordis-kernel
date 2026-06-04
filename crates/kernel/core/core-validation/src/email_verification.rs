use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmailRouteStatus {
    MxFound,
    AddressFallbackFound,
    NoRoute,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailRouteEvidence {
    pub domain_ascii: String,
    pub status: EmailRouteStatus,
    pub mx_hosts: Vec<String>,
    pub address_records_found: bool,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EmailVerificationError {
    #[error("email address is structurally invalid")]
    InvalidEmail,
    #[error("email domain cannot be normalized to ascii")]
    InvalidDomain,
    #[error("dns query failed")]
    DnsQueryFailed,
    #[error("dns response invalid")]
    DnsResponseInvalid,
}

pub trait EmailVerificationPort {
    fn verify_email_route(&self, email: &str)
        -> Result<EmailRouteEvidence, EmailVerificationError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailAttachment {
    pub filename: String,
    pub content_type: String,
    pub content_base64: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailMessage {
    pub from: Option<String>,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub reply_to: Vec<String>,
    pub subject: String,
    pub text_body: Option<String>,
    pub html_body: Option<String>,
    pub attachments: Vec<EmailAttachment>,
}

impl EmailMessage {
    pub fn text(
        to: impl IntoIterator<Item = impl Into<String>>,
        subject: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self {
            from: None,
            to: to.into_iter().map(Into::into).collect(),
            cc: Vec::new(),
            bcc: Vec::new(),
            reply_to: Vec::new(),
            subject: subject.into(),
            text_body: Some(body.into()),
            html_body: None,
            attachments: Vec::new(),
        }
    }

    pub fn html(
        to: impl IntoIterator<Item = impl Into<String>>,
        subject: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self {
            from: None,
            to: to.into_iter().map(Into::into).collect(),
            cc: Vec::new(),
            bcc: Vec::new(),
            reply_to: Vec::new(),
            subject: subject.into(),
            text_body: None,
            html_body: Some(body.into()),
            attachments: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailDeliveryEvidence {
    pub provider: String,
    pub provider_message_id: Option<String>,
    pub accepted_recipients: Vec<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EmailDeliveryError {
    #[error("email message is invalid")]
    InvalidMessage,
    #[error("email recipient is invalid")]
    InvalidRecipient,
    #[error("email provider authentication failed")]
    AuthenticationFailed,
    #[error("email provider rejected the message")]
    ProviderRejected,
    #[error("email provider request failed")]
    ProviderRequestFailed,
}

pub trait EmailDeliveryPort {
    fn send_email(
        &self,
        message: &EmailMessage,
    ) -> Result<EmailDeliveryEvidence, EmailDeliveryError>;
}
