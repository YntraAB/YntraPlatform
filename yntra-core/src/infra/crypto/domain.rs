use crate::infra::errors::YntraError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoDomain {
    KeyStretching,
    WhistleblowerKeyDerivation,
    WhistleblowerAnonymityHash,
    LocalStorageIntegrity,
    UserKeyDerivation,
    PasskeyEnvelopeEncryption,
}

impl CryptoDomain {
    pub fn get_context(&self, version: u32) -> Result<&'static str, YntraError> {
        match (self, version) {
            (Self::KeyStretching, 1) => Ok("Yntra key stretching v1"),
            (Self::WhistleblowerKeyDerivation, 2) => Ok("Yntra whistleblower key derivation v2"),
            (Self::WhistleblowerAnonymityHash, 2) => Ok("Yntra whistleblower reporter anonymity hash v2"),
            (Self::LocalStorageIntegrity, 1) => Ok("Yntra Local Storage Integrity v1"),
            (Self::LocalStorageIntegrity, 2) => Ok("Yntra Local Storage Integrity v2"),
            (Self::UserKeyDerivation, 1) => Ok("Yntra User Key Derivation Context"),
            (Self::PasskeyEnvelopeEncryption, 1) => Ok("Yntra Zero-Copy Passkey Envelope Encryption Key"),
            _ => Err(YntraError::CryptoError(format!(
                "Unsupported cryptographic domain version for {:?}: v{}",
                self, version
            ))),
        }
    }
}
