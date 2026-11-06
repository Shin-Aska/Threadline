use jose_jwk::{Ec, EcCurves, Key, crypto::Error};
use p256::{PublicKey, SecretKey, elliptic_curve::sec1::ToSec1Point};

pub(crate) fn secret_key(key: &Key) -> Result<SecretKey, Error> {
    let Key::Ec(ec) = key else {
        return Err(Error::Unsupported);
    };
    if ec.crv != EcCurves::P256 {
        return Err(Error::AlgMismatch);
    }
    let d = ec.d.as_ref().ok_or(Error::NotPrivate)?;
    if d.len() != 32 || ec.x.len() != 32 || ec.y.len() != 32 {
        return Err(Error::Invalid);
    }
    let secret = SecretKey::from_slice(d).map_err(|_| Error::Invalid)?;
    let mut point = [0u8; 65];
    point[0] = 4;
    point[1..33].copy_from_slice(&ec.x);
    point[33..].copy_from_slice(&ec.y);
    let public = PublicKey::from_sec1_bytes(&point).map_err(|_| Error::Invalid)?;
    if public != secret.public_key() {
        return Err(Error::Invalid);
    }
    Ok(secret)
}

fn public_ec(key: &PublicKey) -> Ec {
    // A P-256 public key is always a finite point with two 32-byte coordinates.
    let point = key.to_sec1_point(false);
    let bytes = point.as_bytes();
    Ec {
        crv: EcCurves::P256,
        x: bytes[1..33].to_vec().into(),
        y: bytes[33..65].to_vec().into(),
        d: None,
    }
}

pub(crate) fn public_jwk(key: &PublicKey) -> Key {
    Key::Ec(public_ec(key))
}

pub(crate) fn private_jwk(key: &SecretKey) -> Key {
    let mut ec = public_ec(&key.public_key());
    ec.d = Some(key.to_bytes().to_vec().into());
    Key::Ec(ec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::elliptic_curve::Generate;

    #[test]
    fn saved_jwk_retains_its_private_key_and_public_coordinates() {
        let original = crate::tests::dpop_key();
        let saved = serde_json::to_string(&original).unwrap();
        let restored = serde_json::from_str(&saved).unwrap();
        let secret = secret_key(&restored).unwrap();
        assert_eq!(private_jwk(&secret), original);
        let Key::Ec(public) = public_jwk(&secret.public_key()) else { panic!("expected EC") };
        assert!(public.d.is_none());
    }

    #[test]
    fn generated_jwk_round_trips_without_regenerating_the_key() {
        let secret = SecretKey::generate();
        let key = private_jwk(&secret);
        let restored = serde_json::from_str(&serde_json::to_string(&key).unwrap()).unwrap();
        assert_eq!(secret_key(&restored).unwrap(), secret);
    }

    #[test]
    fn malformed_or_inconsistent_jwks_are_rejected() {
        let Key::Ec(valid) = crate::tests::dpop_key() else { panic!("expected EC") };
        let mut invalid = Vec::new();
        for scalar in [vec![0; 32], vec![255; 32], vec![1; 31], vec![1; 33]] {
            let mut ec = valid.clone();
            ec.d = Some(scalar.into());
            invalid.push(ec);
        }
        let mut short_x = valid.clone();
        short_x.x = vec![0; 31].into();
        invalid.push(short_x);
        let mut long_y = valid.clone();
        long_y.y = vec![0; 33].into();
        invalid.push(long_y);
        let mut off_curve = valid.clone();
        off_curve.x = vec![0; 32].into();
        off_curve.y = vec![0; 32].into();
        invalid.push(off_curve);
        let mut mismatched = valid.clone();
        mismatched.d = Some(SecretKey::generate().to_bytes().to_vec().into());
        invalid.push(mismatched);
        for ec in invalid {
            assert_eq!(secret_key(&Key::Ec(ec)), Err(Error::Invalid));
        }
        let mut wrong_curve = valid.clone();
        wrong_curve.crv = EcCurves::P384;
        assert_eq!(secret_key(&Key::Ec(wrong_curve)), Err(Error::AlgMismatch));
        let mut public_only = valid;
        public_only.d = None;
        assert_eq!(secret_key(&Key::Ec(public_only)), Err(Error::NotPrivate));
    }
}
