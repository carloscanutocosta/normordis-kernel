use core_validation::{
    EmailDeliveryError, EmailDeliveryEvidence, EmailDeliveryPort, EmailMessage, EmailRouteEvidence,
    EmailRouteStatus, EmailVerificationError, EmailVerificationPort,
};
use serde_json::{json, Value};
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

const TYPE_A: u16 = 1;
const TYPE_MX: u16 = 15;
const TYPE_AAAA: u16 = 28;
const CLASS_IN: u16 = 1;

#[derive(Debug, Clone)]
pub struct DnsEmailVerifier {
    nameserver: SocketAddr,
    timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct GraphEmailSender {
    base_url: String,
    user_id: String,
    bearer_token: String,
    save_to_sent_items: bool,
}

impl GraphEmailSender {
    pub fn new(user_id: impl Into<String>, bearer_token: impl Into<String>) -> Self {
        Self {
            base_url: "https://graph.microsoft.com/v1.0".to_string(),
            user_id: user_id.into(),
            bearer_token: bearer_token.into(),
            save_to_sent_items: true,
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into().trim_end_matches('/').to_string();
        self
    }

    pub fn with_save_to_sent_items(mut self, save_to_sent_items: bool) -> Self {
        self.save_to_sent_items = save_to_sent_items;
        self
    }

    fn endpoint(&self) -> Result<String, EmailDeliveryError> {
        if self.user_id.trim().is_empty() || self.user_id.contains('/') {
            return Err(EmailDeliveryError::InvalidMessage);
        }
        let mailbox = if self.user_id == "me" {
            "me".to_string()
        } else {
            format!("users/{}", self.user_id)
        };
        Ok(format!("{}/{mailbox}/sendMail", self.base_url))
    }
}

impl EmailDeliveryPort for GraphEmailSender {
    fn send_email(
        &self,
        message: &EmailMessage,
    ) -> Result<EmailDeliveryEvidence, EmailDeliveryError> {
        validate_message(message)?;
        if self.bearer_token.trim().is_empty() {
            return Err(EmailDeliveryError::AuthenticationFailed);
        }

        let payload = serde_json::to_string(&build_graph_payload(message, self.save_to_sent_items))
            .map_err(|_| EmailDeliveryError::InvalidMessage)?;

        let response = ureq::post(&self.endpoint()?)
            .set("Authorization", &format!("Bearer {}", self.bearer_token))
            .set("Content-Type", "application/json")
            .send_string(&payload);

        match response {
            Ok(response) if (200..300).contains(&response.status()) => Ok(EmailDeliveryEvidence {
                provider: "microsoft-graph".to_string(),
                provider_message_id: response.header("x-ms-request-id").map(str::to_string),
                accepted_recipients: all_recipients(message),
            }),
            Ok(response) if response.status() == 401 || response.status() == 403 => {
                Err(EmailDeliveryError::AuthenticationFailed)
            }
            Ok(_) => Err(EmailDeliveryError::ProviderRejected),
            Err(ureq::Error::Status(status, _)) if status == 401 || status == 403 => {
                Err(EmailDeliveryError::AuthenticationFailed)
            }
            Err(ureq::Error::Status(_, _)) => Err(EmailDeliveryError::ProviderRejected),
            Err(_) => Err(EmailDeliveryError::ProviderRequestFailed),
        }
    }
}

impl DnsEmailVerifier {
    pub fn new(nameserver: SocketAddr) -> Self {
        Self {
            nameserver,
            timeout: Duration::from_secs(3),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    fn query(&self, domain: &str, record_type: u16) -> Result<DnsRecords, EmailVerificationError> {
        let query = build_query(domain, record_type)?;
        let socket =
            UdpSocket::bind("0.0.0.0:0").map_err(|_| EmailVerificationError::DnsQueryFailed)?;
        socket
            .set_read_timeout(Some(self.timeout))
            .map_err(|_| EmailVerificationError::DnsQueryFailed)?;
        socket
            .set_write_timeout(Some(self.timeout))
            .map_err(|_| EmailVerificationError::DnsQueryFailed)?;
        socket
            .send_to(&query, self.nameserver)
            .map_err(|_| EmailVerificationError::DnsQueryFailed)?;

        let mut buffer = [0_u8; 1500];
        let (read, _) = socket
            .recv_from(&mut buffer)
            .map_err(|_| EmailVerificationError::DnsQueryFailed)?;
        parse_response(&buffer[..read])
    }
}

impl EmailVerificationPort for DnsEmailVerifier {
    fn verify_email_route(
        &self,
        email: &str,
    ) -> Result<EmailRouteEvidence, EmailVerificationError> {
        if !support_normalization::is_valid_email(email) {
            return Err(EmailVerificationError::InvalidEmail);
        }
        let (_, domain) = email
            .trim()
            .split_once('@')
            .ok_or(EmailVerificationError::InvalidEmail)?;
        let domain_ascii = support_normalization::normalize_domain_to_ascii(domain)
            .ok_or(EmailVerificationError::InvalidDomain)?;

        let mx = self.query(&domain_ascii, TYPE_MX)?;
        if !mx.mx_hosts.is_empty() {
            return Ok(EmailRouteEvidence {
                domain_ascii,
                status: EmailRouteStatus::MxFound,
                mx_hosts: mx.mx_hosts,
                address_records_found: false,
            });
        }

        let a_found = self.query(&domain_ascii, TYPE_A)?.address_records_found;
        let aaaa_found = self.query(&domain_ascii, TYPE_AAAA)?.address_records_found;
        let address_records_found = a_found || aaaa_found;
        Ok(EmailRouteEvidence {
            domain_ascii,
            status: if address_records_found {
                EmailRouteStatus::AddressFallbackFound
            } else {
                EmailRouteStatus::NoRoute
            },
            mx_hosts: Vec::new(),
            address_records_found,
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct DnsRecords {
    mx_hosts: Vec<String>,
    address_records_found: bool,
}

fn build_query(domain: &str, record_type: u16) -> Result<Vec<u8>, EmailVerificationError> {
    let mut query = Vec::with_capacity(512);
    query.extend_from_slice(&0x4e4f_u16.to_be_bytes());
    query.extend_from_slice(&0x0100_u16.to_be_bytes());
    query.extend_from_slice(&1_u16.to_be_bytes());
    query.extend_from_slice(&0_u16.to_be_bytes());
    query.extend_from_slice(&0_u16.to_be_bytes());
    query.extend_from_slice(&0_u16.to_be_bytes());
    encode_name(domain, &mut query)?;
    query.extend_from_slice(&record_type.to_be_bytes());
    query.extend_from_slice(&CLASS_IN.to_be_bytes());
    Ok(query)
}

fn encode_name(domain: &str, out: &mut Vec<u8>) -> Result<(), EmailVerificationError> {
    for label in domain.split('.') {
        if label.is_empty() || label.len() > 63 {
            return Err(EmailVerificationError::DnsResponseInvalid);
        }
        out.push(label.len() as u8);
        out.extend_from_slice(label.as_bytes());
    }
    out.push(0);
    Ok(())
}

fn parse_response(data: &[u8]) -> Result<DnsRecords, EmailVerificationError> {
    if data.len() < 12 {
        return Err(EmailVerificationError::DnsResponseInvalid);
    }
    let flags = read_u16(data, 2)?;
    if flags & 0x0200 != 0 {
        return Err(EmailVerificationError::DnsResponseInvalid);
    }
    let rcode = flags & 0x000f;
    if rcode != 0 && rcode != 3 {
        return Err(EmailVerificationError::DnsQueryFailed);
    }

    let qdcount = read_u16(data, 4)? as usize;
    let ancount = read_u16(data, 6)? as usize;
    let mut offset = 12;
    for _ in 0..qdcount {
        skip_name(data, &mut offset)?;
        offset = offset
            .checked_add(4)
            .ok_or(EmailVerificationError::DnsResponseInvalid)?;
        if offset > data.len() {
            return Err(EmailVerificationError::DnsResponseInvalid);
        }
    }

    let mut records = DnsRecords::default();
    for _ in 0..ancount {
        skip_name(data, &mut offset)?;
        let record_type = read_u16(data, offset)?;
        let class = read_u16(data, offset + 2)?;
        let rdlen = read_u16(data, offset + 8)? as usize;
        offset = offset
            .checked_add(10)
            .ok_or(EmailVerificationError::DnsResponseInvalid)?;
        let rdata_end = offset
            .checked_add(rdlen)
            .ok_or(EmailVerificationError::DnsResponseInvalid)?;
        if rdata_end > data.len() {
            return Err(EmailVerificationError::DnsResponseInvalid);
        }

        if class == CLASS_IN {
            match record_type {
                TYPE_MX if rdlen >= 3 => {
                    let mut mx_offset = offset + 2;
                    let host = read_name(data, &mut mx_offset)?;
                    records.mx_hosts.push(host);
                }
                TYPE_A if rdlen == 4 => records.address_records_found = true,
                TYPE_AAAA if rdlen == 16 => records.address_records_found = true,
                _ => {}
            }
        }
        offset = rdata_end;
    }
    records.mx_hosts.sort();
    records.mx_hosts.dedup();
    Ok(records)
}

fn read_name(data: &[u8], offset: &mut usize) -> Result<String, EmailVerificationError> {
    let mut labels = Vec::new();
    let mut cursor = *offset;
    let mut jumped = false;
    let mut jumps = 0;

    loop {
        if cursor >= data.len() {
            return Err(EmailVerificationError::DnsResponseInvalid);
        }
        let len = data[cursor];
        if len & 0xc0 == 0xc0 {
            if cursor + 1 >= data.len() {
                return Err(EmailVerificationError::DnsResponseInvalid);
            }
            let ptr = (((len & 0x3f) as usize) << 8) | data[cursor + 1] as usize;
            if !jumped {
                *offset = cursor + 2;
            }
            cursor = ptr;
            jumped = true;
            jumps += 1;
            if jumps > 16 {
                return Err(EmailVerificationError::DnsResponseInvalid);
            }
            continue;
        }
        if len == 0 {
            if !jumped {
                *offset = cursor + 1;
            }
            break;
        }
        if len > 63 {
            return Err(EmailVerificationError::DnsResponseInvalid);
        }
        cursor += 1;
        let end = cursor
            .checked_add(len as usize)
            .ok_or(EmailVerificationError::DnsResponseInvalid)?;
        if end > data.len() {
            return Err(EmailVerificationError::DnsResponseInvalid);
        }
        labels.push(
            std::str::from_utf8(&data[cursor..end])
                .map_err(|_| EmailVerificationError::DnsResponseInvalid)?
                .to_ascii_lowercase(),
        );
        cursor = end;
    }
    Ok(labels.join("."))
}

fn skip_name(data: &[u8], offset: &mut usize) -> Result<(), EmailVerificationError> {
    read_name(data, offset).map(|_| ())
}

fn read_u16(data: &[u8], offset: usize) -> Result<u16, EmailVerificationError> {
    let bytes = data
        .get(offset..offset + 2)
        .ok_or(EmailVerificationError::DnsResponseInvalid)?;
    Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
}

fn validate_message(message: &EmailMessage) -> Result<(), EmailDeliveryError> {
    if message.subject.trim().is_empty()
        || message
            .text_body
            .as_deref()
            .is_none_or(|body| body.trim().is_empty())
            && message
                .html_body
                .as_deref()
                .is_none_or(|body| body.trim().is_empty())
        || all_recipients(message).is_empty()
    {
        return Err(EmailDeliveryError::InvalidMessage);
    }

    if all_delivery_addresses(message)
        .iter()
        .any(|email| !support_normalization::is_valid_email(email))
    {
        return Err(EmailDeliveryError::InvalidRecipient);
    }

    if message.attachments.iter().any(|attachment| {
        attachment.filename.trim().is_empty()
            || attachment.content_type.trim().is_empty()
            || attachment.content_base64.trim().is_empty()
    }) {
        return Err(EmailDeliveryError::InvalidMessage);
    }

    Ok(())
}

fn all_recipients(message: &EmailMessage) -> Vec<String> {
    message
        .to
        .iter()
        .chain(message.cc.iter())
        .chain(message.bcc.iter())
        .cloned()
        .collect()
}

fn all_delivery_addresses(message: &EmailMessage) -> Vec<String> {
    message
        .to
        .iter()
        .chain(message.cc.iter())
        .chain(message.bcc.iter())
        .chain(message.reply_to.iter())
        .cloned()
        .collect()
}

fn build_graph_payload(message: &EmailMessage, save_to_sent_items: bool) -> Value {
    let body = if let Some(html) = message.html_body.as_deref() {
        json!({ "contentType": "HTML", "content": html })
    } else {
        json!({ "contentType": "Text", "content": message.text_body.as_deref().unwrap_or("") })
    };

    let mut graph_message = json!({
        "subject": message.subject,
        "body": body,
        "toRecipients": graph_recipients(&message.to),
        "ccRecipients": graph_recipients(&message.cc),
        "bccRecipients": graph_recipients(&message.bcc),
        "replyTo": graph_recipients(&message.reply_to),
    });

    if !message.attachments.is_empty() {
        graph_message["attachments"] = Value::Array(
            message
                .attachments
                .iter()
                .map(|attachment| {
                    json!({
                        "@odata.type": "#microsoft.graph.fileAttachment",
                        "name": attachment.filename,
                        "contentType": attachment.content_type,
                        "contentBytes": attachment.content_base64,
                    })
                })
                .collect(),
        );
    }

    json!({
        "message": graph_message,
        "saveToSentItems": save_to_sent_items,
    })
}

fn graph_recipients(recipients: &[String]) -> Value {
    Value::Array(
        recipients
            .iter()
            .map(|address| json!({ "emailAddress": { "address": address } }))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_encodes_domain_labels() {
        let query = build_query("example.com", TYPE_MX).unwrap();
        assert!(query.windows(13).any(|w| w == b"\x07example\x03com\0"));
    }

    #[test]
    fn parser_reads_mx_answer() {
        let mut packet = build_query("example.com", TYPE_MX).unwrap();
        packet[2] = 0x81;
        packet[3] = 0x80;
        packet[7] = 1;
        packet.extend_from_slice(&[0xc0, 0x0c]);
        packet.extend_from_slice(&TYPE_MX.to_be_bytes());
        packet.extend_from_slice(&CLASS_IN.to_be_bytes());
        packet.extend_from_slice(&60_u32.to_be_bytes());
        let mut rdata = Vec::new();
        rdata.extend_from_slice(&10_u16.to_be_bytes());
        encode_name("mail.example.com", &mut rdata).unwrap();
        packet.extend_from_slice(&(rdata.len() as u16).to_be_bytes());
        packet.extend_from_slice(&rdata);

        let records = parse_response(&packet).unwrap();
        assert_eq!(records.mx_hosts, vec!["mail.example.com"]);
        assert!(!records.address_records_found);
    }

    #[test]
    fn parser_reads_a_answer() {
        let mut packet = build_query("example.com", TYPE_A).unwrap();
        packet[2] = 0x81;
        packet[3] = 0x80;
        packet[7] = 1;
        packet.extend_from_slice(&[0xc0, 0x0c]);
        packet.extend_from_slice(&TYPE_A.to_be_bytes());
        packet.extend_from_slice(&CLASS_IN.to_be_bytes());
        packet.extend_from_slice(&60_u32.to_be_bytes());
        packet.extend_from_slice(&4_u16.to_be_bytes());
        packet.extend_from_slice(&[192, 0, 2, 1]);

        let records = parse_response(&packet).unwrap();
        assert!(records.address_records_found);
    }

    #[test]
    fn graph_payload_builds_text_message() {
        let message = EmailMessage::text(["user@example.pt"], "Assunto", "Corpo");
        let payload = build_graph_payload(&message, true);
        assert_eq!(payload["message"]["subject"], "Assunto");
        assert_eq!(payload["message"]["body"]["contentType"], "Text");
        assert_eq!(
            payload["message"]["toRecipients"][0]["emailAddress"]["address"],
            "user@example.pt"
        );
        assert_eq!(payload["saveToSentItems"], true);
    }

    #[test]
    fn graph_sender_rejects_invalid_message_before_network() {
        let sender = GraphEmailSender::new("me", "token");
        let message = EmailMessage::text(["bad email"], "Assunto", "Corpo");
        assert_eq!(
            sender.send_email(&message).unwrap_err(),
            EmailDeliveryError::InvalidRecipient
        );
    }
}
