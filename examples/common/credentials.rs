use std::env;

use native_tls::{Certificate, Identity};

pub fn identity() -> Identity {
    let pw = env::var("DEVICE_ADVISOR_PASSWORD").unwrap_or_default();
    Identity::from_pkcs12(include_bytes!("../secrets/identity.pfx"), pw.as_str()).unwrap()
}

pub fn claim_identity() -> Identity {
    Identity::from_pkcs12(include_bytes!("../secrets/claim_identity.pfx"), "").unwrap()
}

pub fn root_ca() -> Certificate {
    Certificate::from_pem(include_bytes!("../secrets/root-ca.pem")).unwrap()
}

pub const HOSTNAME: Option<&'static str> = option_env!("AWS_HOSTNAME");
