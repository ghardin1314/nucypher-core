use alloc::boxed::Box;

use serde::{Deserialize, Serialize};
use umbral_pre::{
    decrypt_original, encrypt, Capsule, DeserializableFromArray, EncryptionError, KeyFrag,
    PublicKey, SecretKey, SerializableToArray, Signature, Signer, VerifiedKeyFrag,
};

use crate::hrac::HRAC;
use crate::serde::{standard_deserialize, standard_serialize};

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizedKeyFrag {
    signature: Signature,
    kfrag: KeyFrag,
}

impl AuthorizedKeyFrag {
    pub(crate) fn new(signer: &Signer, hrac: &HRAC, verified_kfrag: &VerifiedKeyFrag) -> Self {
        // Alice makes plain to Ursula that, upon decrypting this message,
        // this particular KFrag is authorized for use in the policy identified by this HRAC.

        // TODO (rust-umbral#73): add VerifiedKeyFrag::unverify()?
        let kfrag = KeyFrag::from_array(&verified_kfrag.to_array()).unwrap();

        let signature = signer.sign(&[hrac.as_ref(), &kfrag.to_array()].concat());

        Self { signature, kfrag }
    }

    pub(crate) fn verify(
        &self,
        hrac: &HRAC,
        publisher_verifying_key: &PublicKey,
    ) -> Option<VerifiedKeyFrag> {
        if !self.signature.verify(
            publisher_verifying_key,
            &[hrac.as_ref(), &self.kfrag.to_array()].concat(),
        ) {
            return None;
        }

        // Ursula has no side channel to get the KeyFrag author's key,
        // so verifying the keyfrag is useless.
        // TODO (rust-umbral#73): assuming here that VerifiedKeyFrag and KeyFrag have the same byte representation;
        // would it be more clear if `kfrag` had some method like `force_verify()`?
        VerifiedKeyFrag::from_verified_bytes(&self.kfrag.to_array()).ok()
    }
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedKeyFrag {
    capsule: Capsule,
    ciphertext: Box<[u8]>,
}

impl EncryptedKeyFrag {
    pub fn new(
        recipient_key: &PublicKey,
        authorized_kfrag: &AuthorizedKeyFrag,
    ) -> Result<Self, EncryptionError> {
        // Using Umbral for asymmetric encryption here for simplicity,
        // even though we do not plan to re-encrypt the capsule.
        let (capsule, ciphertext) = encrypt(recipient_key, &standard_serialize(&authorized_kfrag))?;
        Ok(Self {
            capsule,
            ciphertext,
        })
    }

    pub fn decrypt(self, sk: &SecretKey) -> Option<AuthorizedKeyFrag> {
        let auth_kfrag_bytes = decrypt_original(sk, &self.capsule, self.ciphertext).unwrap();
        Some(standard_deserialize::<AuthorizedKeyFrag>(&auth_kfrag_bytes))
    }
}