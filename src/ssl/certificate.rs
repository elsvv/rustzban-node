use openssl::{
    asn1::Asn1Time,
    bn::{BigNum, MsbOption},
    hash::MessageDigest,
    pkey::PKey,
    rsa::Rsa,
    x509::{X509NameBuilder, X509},
};
use std::fmt;

/// Ошибки при генерации сертификатов
#[derive(Debug)]
pub enum CertificateError {
    OpenSSLError(openssl::error::ErrorStack),
    InvalidData(String),
}

impl fmt::Display for CertificateError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CertificateError::OpenSSLError(e) => write!(f, "OpenSSL error: {}", e),
            CertificateError::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
        }
    }
}

impl std::error::Error for CertificateError {}

impl From<openssl::error::ErrorStack> for CertificateError {
    fn from(err: openssl::error::ErrorStack) -> Self {
        CertificateError::OpenSSLError(err)
    }
}

/// Результат генерации сертификата
#[derive(Debug, Clone)]
pub struct CertificatePair {
    pub cert: String,  // PEM формат сертификата
    pub key: String,   // PEM формат приватного ключа
}

/// Генерирует SSL сертификат и приватный ключ
/// Идентично функции generate_certificate() из certificate.py Python версии
pub fn generate_certificate() -> Result<CertificatePair, CertificateError> {
    // Генерируем RSA ключ 4096 бит (как в Python версии)
    let rsa = Rsa::generate(4096)?;
    let key_pair = PKey::from_rsa(rsa)?;
    
    // Создаем новый X509 сертификат
    let mut cert_builder = X509::builder()?;
    
    // Устанавливаем версию сертификата (v3)
    cert_builder.set_version(2)?;
    
    // Генерируем серийный номер
    let mut serial = BigNum::new()?;
    serial.rand(159, MsbOption::MAYBE_ZERO, false)?;
    let asn1_serial = serial.to_asn1_integer()?;
    cert_builder.set_serial_number(&asn1_serial)?;
    
    // Устанавливаем subject и issuer (CN="Gozargah" как в Python версии)
    let mut name_builder = X509NameBuilder::new()?;
    name_builder.append_entry_by_text("CN", "Gozargah")?;
    let name = name_builder.build();
    
    cert_builder.set_subject_name(&name)?;
    cert_builder.set_issuer_name(&name)?; // Self-signed
    
    // Устанавливаем публичный ключ
    cert_builder.set_pubkey(&key_pair)?;
    
    // Устанавливаем время действия сертификата
    // notBefore = сейчас (gmtime_adj_notBefore(0) в Python)
    let not_before = Asn1Time::days_from_now(0)?;
    cert_builder.set_not_before(&not_before)?;
    
    // notAfter = 100 лет (100*365*24*60*60 секунд в Python)
    let not_after = Asn1Time::days_from_now(100 * 365)?;
    cert_builder.set_not_after(&not_after)?;
    
    // Подписываем сертификат приватным ключом с SHA512 (как в Python версии)
    cert_builder.sign(&key_pair, MessageDigest::sha512())?;
    
    let cert = cert_builder.build();
    
    // Конвертируем в PEM формат (как в Python версии)
    let cert_pem = String::from_utf8(cert.to_pem()?)
        .map_err(|e| CertificateError::InvalidData(format!("Invalid UTF-8 in cert: {}", e)))?;
    
    let key_pem = String::from_utf8(key_pair.private_key_to_pem_pkcs8()?)
        .map_err(|e| CertificateError::InvalidData(format!("Invalid UTF-8 in key: {}", e)))?;
    
    Ok(CertificatePair {
        cert: cert_pem,
        key: key_pem,
    })
}

/// Сохраняет сертификат и ключ в файлы
pub fn save_certificate_files(
    cert_pair: &CertificatePair,
    cert_file_path: &str,
    key_file_path: &str,
) -> Result<(), std::io::Error> {
    use std::fs;
    
    // Создаем директории если не существуют
    if let Some(parent) = std::path::Path::new(cert_file_path).parent() {
        fs::create_dir_all(parent)?;
    }
    if let Some(parent) = std::path::Path::new(key_file_path).parent() {
        fs::create_dir_all(parent)?;
    }
    
    // Сохраняем файлы
    fs::write(cert_file_path, &cert_pair.cert)?;
    fs::write(key_file_path, &cert_pair.key)?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use openssl::x509::X509;
    use openssl::pkey::PKey;
    
    #[test]
    fn test_generate_certificate() {
        let cert_pair = generate_certificate().unwrap();
        
        // Проверяем что сертификат и ключ не пустые
        assert!(!cert_pair.cert.is_empty());
        assert!(!cert_pair.key.is_empty());
        
        // Проверяем что сертификат валиден
        let cert = X509::from_pem(cert_pair.cert.as_bytes()).unwrap();
        let key = PKey::private_key_from_pem(cert_pair.key.as_bytes()).unwrap();
        
        // Проверяем CN="Gozargah" (как в Python версии)
        let subject = cert.subject_name();
        let cn_entries: Vec<_> = subject.entries_by_nid(openssl::nid::Nid::COMMONNAME).collect();
        assert_eq!(cn_entries.len(), 1);
        assert_eq!(cn_entries[0].data().as_utf8().unwrap().to_string(), "Gozargah");
        
        // Проверяем что ключ соответствует сертификату
        assert!(cert.public_key().unwrap().public_eq(&key));
    }
    
    #[test]
    fn test_certificate_pem_format() {
        let cert_pair = generate_certificate().unwrap();
        
        // Проверяем PEM формат сертификата
        assert!(cert_pair.cert.starts_with("-----BEGIN CERTIFICATE-----"));
        assert!(cert_pair.cert.ends_with("-----END CERTIFICATE-----\n"));
        
        // Проверяем PEM формат ключа
        assert!(cert_pair.key.starts_with("-----BEGIN PRIVATE KEY-----"));
        assert!(cert_pair.key.ends_with("-----END PRIVATE KEY-----\n"));
    }
    
    #[test]
    fn test_save_certificate_files() {
        use tempfile::tempdir;
        
        let cert_pair = generate_certificate().unwrap();
        let temp_dir = tempdir().unwrap();
        
        let cert_path = temp_dir.path().join("cert.pem");
        let key_path = temp_dir.path().join("key.pem");
        
        save_certificate_files(
            &cert_pair,
            cert_path.to_str().unwrap(),
            key_path.to_str().unwrap(),
        ).unwrap();
        
        // Проверяем что файлы созданы и содержат правильные данные
        let saved_cert = std::fs::read_to_string(&cert_path).unwrap();
        let saved_key = std::fs::read_to_string(&key_path).unwrap();
        
        assert_eq!(saved_cert, cert_pair.cert);
        assert_eq!(saved_key, cert_pair.key);
    }
} 