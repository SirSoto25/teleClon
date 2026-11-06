//! Diffie-Hellman de chats secretos: validación de `messages.getDhConfig`,
//! generación de exponentes secretos y cálculo de `g_a`, `g_b` y la clave.
//!
//! Reparto de aritmética:
//! - Valores públicos (`p`, `g`, `g_a`, `g_b`): `num-bigint` (tiempo variable, no hay secretos).
//! - Exponenciación con exponente secreto (`a`, `b`): `crypto-bigint` en forma de
//!   Montgomery con `pow` de tiempo constante respecto al exponente.

use core::fmt;

use crypto_bigint::modular::{FixedMontyForm, FixedMontyParams};
use crypto_bigint::{Odd, U2048};
use num_bigint::BigUint;
use zeroize::{Zeroize, Zeroizing};

use crate::entropy::Entropy;
use crate::error::{Result, SecretError};
use crate::key::{KEY_LEN, SecretKey};

const LIMBS: usize = U2048::LIMBS;

/// Rondas de Miller-Rabin con bases aleatorias por número (error ≤ 4^-32 = 2^-64
/// incluso con un `p` elegido por un adversario).
pub const MILLER_RABIN_ROUNDS: usize = 32;

/// Parámetros DH ya validados. Validar es caro (~centenares de ms): la
/// especificación pide cachear el resultado junto con `version`.
#[derive(Clone)]
pub struct DhParams {
    g: u32,
    p: [u8; KEY_LEN],
    version: i32,
    p_big: BigUint,
    monty: FixedMontyParams<LIMBS>,
}

impl fmt::Debug for DhParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DhParams").field("g", &self.g).field("version", &self.version).finish_non_exhaustive()
    }
}

impl DhParams {
    /// Valida `g` y `p` tal como los devuelve `messages.getDhConfig`:
    /// - `p` es un primo seguro de 2048 bits: `2^2047 < p < 2^2048`, `p` y `(p-1)/2` primos;
    /// - `g ∈ {2..7}` genera el subgrupo de orden primo `(p-1)/2` (condición por residuo de `p`).
    pub fn validate(g: i32, p: &[u8], version: i32, entropy: &mut dyn Entropy) -> Result<Self> {
        if p.len() != KEY_LEN {
            return Err(SecretError::InvalidDhConfig("p no mide 256 bytes"));
        }
        if p[0] & 0x80 == 0 {
            return Err(SecretError::InvalidDhConfig("p no está en (2^2047, 2^2048)"));
        }
        if p[KEY_LEN - 1] & 1 == 0 {
            return Err(SecretError::InvalidDhConfig("p es par"));
        }
        let g = u32::try_from(g).map_err(|_| SecretError::InvalidDhConfig("g negativo"))?;
        let p_big = BigUint::from_bytes_be(p);
        let residue_ok = |m: u32, allowed: &[u32]| {
            let r = (&p_big % m).to_u32_digits().first().copied().unwrap_or(0);
            allowed.contains(&r)
        };
        let g_ok = match g {
            2 => residue_ok(8, &[7]),
            3 => residue_ok(3, &[2]),
            4 => true,
            5 => residue_ok(5, &[1, 4]),
            6 => residue_ok(24, &[19, 23]),
            7 => residue_ok(7, &[3, 5, 6]),
            _ => return Err(SecretError::InvalidDhConfig("g fuera de 2..7")),
        };
        if !g_ok {
            return Err(SecretError::InvalidDhConfig("g no genera el subgrupo de orden (p-1)/2"));
        }
        let q: BigUint = (&p_big - 1u32) >> 1;
        // Primero lo barato en ambos números; luego las rondas completas.
        if !trial_division_ok(&p_big) || !trial_division_ok(&q) {
            return Err(SecretError::InvalidDhConfig("p no es un primo seguro"));
        }
        if !is_probable_prime(&p_big, 1, entropy)? || !is_probable_prime(&q, 1, entropy)? {
            return Err(SecretError::InvalidDhConfig("p no es un primo seguro"));
        }
        if !is_probable_prime(&p_big, MILLER_RABIN_ROUNDS, entropy)?
            || !is_probable_prime(&q, MILLER_RABIN_ROUNDS, entropy)?
        {
            return Err(SecretError::InvalidDhConfig("p no es un primo seguro"));
        }
        let mut p_arr = [0u8; KEY_LEN];
        p_arr.copy_from_slice(p);
        let p_ct = Odd::new(U2048::from_be_slice(p))
            .into_option()
            .ok_or(SecretError::InvalidDhConfig("p es par"))?;
        Ok(Self { g, p: p_arr, version, p_big, monty: FixedMontyParams::new_vartime(p_ct) })
    }

    /// Generador `g`.
    pub fn g(&self) -> u32 {
        self.g
    }

    /// Primo `p` en big-endian.
    pub fn p(&self) -> &[u8; KEY_LEN] {
        &self.p
    }

    /// `version` de `messages.dhConfig` (para pedir `dhConfigNotModified`).
    pub fn version(&self) -> i32 {
        self.version
    }

    /// Comprueba un valor público recibido (`g_a` o `g_b`):
    /// `1 < x < p-1` y además `2^(2048-64) ≤ x ≤ p - 2^(2048-64)`.
    pub fn check_public(&self, value: &[u8]) -> Result<()> {
        if value.is_empty() || value.len() > KEY_LEN {
            return Err(SecretError::InvalidPublicValue);
        }
        let x = BigUint::from_bytes_be(value);
        let one = BigUint::from(1u32);
        let p_minus_1 = &self.p_big - 1u32;
        if x <= one || x >= p_minus_1 {
            return Err(SecretError::InvalidPublicValue);
        }
        let bound = BigUint::from(1u32) << (2048 - 64);
        if x < bound || x > &self.p_big - &bound {
            return Err(SecretError::InvalidPublicValue);
        }
        Ok(())
    }

    fn pow_secret(&self, base: &[u8; KEY_LEN], exp: &U2048) -> Zeroizing<[u8; KEY_LEN]> {
        let base = U2048::from_be_slice(base);
        let mut r = FixedMontyForm::new(&base, &self.monty).pow(exp);
        let mut value = r.retrieve();
        r.zeroize();
        let enc = value.to_be_bytes();
        value.zeroize();
        let mut out = Zeroizing::new([0u8; KEY_LEN]);
        out.copy_from_slice(enc.as_ref());
        out
    }
}

/// Rellena con ceros a la izquierda hasta 256 bytes.
pub(crate) fn left_pad(value: &[u8]) -> Result<[u8; KEY_LEN]> {
    if value.len() > KEY_LEN {
        return Err(SecretError::InvalidPublicValue);
    }
    let mut out = [0u8; KEY_LEN];
    out[KEY_LEN - value.len()..].copy_from_slice(value);
    Ok(out)
}

/// Exponente secreto `a` o `b` (2048 bits). Se borra al soltarse.
pub struct DhSecret {
    exp: U2048,
    public: [u8; KEY_LEN],
}

impl Drop for DhSecret {
    fn drop(&mut self) {
        self.exp.zeroize();
    }
}

impl fmt::Debug for DhSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DhSecret(<censurado>)")
    }
}

impl DhSecret {
    /// Genera un exponente de 2048 bits mezclando (XOR) aleatoriedad local con la
    /// opcional del servidor (`messages.getDhConfig` con `random_length`). La del
    /// servidor sola no es segura; la local siempre se usa.
    ///
    /// Repite si el `g^x` resultante no supera `check_public` (probabilidad ~2^-63).
    pub fn generate(params: &DhParams, entropy: &mut dyn Entropy, server_random: &[u8]) -> Result<Self> {
        for _ in 0..16 {
            let mut raw = Zeroizing::new([0u8; KEY_LEN]);
            entropy.fill(raw.as_mut())?;
            for (dst, src) in raw.iter_mut().zip(server_random.iter()) {
                *dst ^= *src;
            }
            let secret = Self::from_exponent_bytes(params, &raw);
            if params.check_public(&secret.public).is_ok() {
                return Ok(secret);
            }
        }
        Err(SecretError::Entropy)
    }

    /// Construye el secreto desde bytes fijos (vectores de prueba).
    pub(crate) fn from_exponent_bytes(params: &DhParams, raw: &[u8; KEY_LEN]) -> Self {
        let exp = U2048::from_be_slice(raw);
        let mut g = [0u8; KEY_LEN];
        g[KEY_LEN - 4..].copy_from_slice(&params.g.to_be_bytes());
        let public = *params.pow_secret(&g, &exp);
        Self { exp, public }
    }

    /// Valor público `g^x mod p` (256 bytes big-endian).
    pub fn public(&self) -> &[u8; KEY_LEN] {
        &self.public
    }

    /// Clave compartida `peer^x mod p`, tras validar el valor público del otro extremo.
    pub fn shared_key(&self, params: &DhParams, peer_public: &[u8]) -> Result<SecretKey> {
        params.check_public(peer_public)?;
        let peer = left_pad(peer_public)?;
        let k = params.pow_secret(&peer, &self.exp);
        Ok(SecretKey::from_bytes(*k))
    }
}

// ---- primalidad (solo valores públicos) ----

fn small_primes() -> Vec<u32> {
    const LIMIT: usize = 2000;
    let mut sieve = vec![true; LIMIT];
    sieve[0] = false;
    sieve[1] = false;
    let mut i = 2;
    while i * i < LIMIT {
        if sieve[i] {
            let mut j = i * i;
            while j < LIMIT {
                sieve[j] = false;
                j += i;
            }
        }
        i += 1;
    }
    sieve.iter().enumerate().filter(|(_, is)| **is).map(|(n, _)| n as u32).collect()
}

fn trial_division_ok(n: &BigUint) -> bool {
    for sp in small_primes() {
        let r = n % sp;
        if r == BigUint::ZERO {
            return *n == BigUint::from(sp);
        }
    }
    true
}

/// Miller-Rabin con `rounds` bases aleatorias uniformes en `[2, n-2]`.
pub(crate) fn is_probable_prime(n: &BigUint, rounds: usize, entropy: &mut dyn Entropy) -> Result<bool> {
    let one = BigUint::from(1u32);
    let two = BigUint::from(2u32);
    if *n < two {
        return Ok(false);
    }
    if *n < BigUint::from(4u32) {
        return Ok(true);
    }
    if (n % 2u32) == BigUint::ZERO {
        return Ok(false);
    }
    let n_minus_1 = n - &one;
    let s = n_minus_1.trailing_zeros().unwrap_or(0);
    let d = &n_minus_1 >> s;
    let byte_len = n.bits().div_ceil(8) as usize + 8; // 64 bits extra: sesgo de la reducción despreciable
    let span = n - 3u32; // bases en [2, n-2]
    let mut buf = vec![0u8; byte_len];
    'rounds: for _ in 0..rounds {
        entropy.fill(&mut buf)?;
        let a = BigUint::from_bytes_be(&buf) % &span + &two;
        let mut x = a.modpow(&d, n);
        if x == one || x == n_minus_1 {
            continue;
        }
        for _ in 1..s {
            x = (&x * &x) % n;
            if x == n_minus_1 {
                continue 'rounds;
            }
        }
        return Ok(false);
    }
    Ok(true)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::testing::{SeededEntropy, hex, rfc3526_p};
    use sha2::{Digest, Sha256};

    /// `p` de 2048 bits que el servidor de Telegram devuelve en `getDhConfig` (valor
    /// público, citado en la documentación de MTProto). Solo se usa para comprobar
    /// que la validación lo acepta con `g = 3` y lo rechaza con `g` incompatibles.
    const TG_P: &str = concat!(
        "C71CAEB9C6B1C9048E6C522F70F13F73980D40238E3E21C14934D037563D930F48198A0AA7C14058229493D22530F4DB",
        "FA336F6E0AC925139543AED44CCE7C3720FD51F69458705AC68CD4FE6B6B13ABDC9746512969328454F18FAF8C595F64",
        "2477FE96BB2A941D5BCD1D4AC8CC49880708FA9B378E3C4F3A9060BEE67CF9A4A4A695811051907E162753B56B0F6B41",
        "0DBA74D8A84B2A14B3144E0EF1284754FD17ED950D5965B4B9DD46582DB1178D169C6BC465B0D6FF9CA3928FEF5B9AE4",
        "E418FC15E83EBEA0F87FA9FF5EED70050DED2849F47BF959D956850CE929851F0D8115F635B105EE2E4E15D04B2454BF",
        "6F4FADF034B10403119CD8E3B92FCC5B",
    );

    // Vector calculado con una implementación independiente en Python (`pow` nativo + hashlib):
    // a = SHA256("tc-secret a") repetido 8 veces, b = SHA256("tc-secret b") ×8, p = RFC 3526, g = 2.
    const GA: &str = "9bf71d02d589c8b4f581a094402f8fdf7400218cf6a9bba13fc725626600dc2edc019e16931b75beac00153b15fa343177b6cce34206954f713745cce7e8f7766687b92816e0d77f5f029bb59e757a417ca715a01c9a7a070f8f7cb70f6ba9bba559826f252d77f12597b3c35c2782f858fb58e0ba0ef6f2c94f5f41b3cdd9915284186217dc63c31ef899f9f11cb981fd88676f1c8ccf79efc6e43667ecea6488335c4af467de8fd3bb3b4975f3d746e36db89d07243885fa05de8cb3c34f743aeba40552f58e075f95f9961b9411ae8462df34ce89d8d2d56b5b642ccc77f6f59f508b8a73d40ca836ca14f7afa18161497ee09cff673a4f924d8fa0a4c624";
    const GB: &str = "f683efd7bb500e31b3fa84787f18a60ed69330f8d5126af73f9efd5d9fcf7402617234e43bea336e894ce775e62fc6015ad7e881c6fff43ac1e2d208a57e1a397641873ce7e5656065f029ca5d4da821a4f30c752b5684b5a76aca33d387ca930d4277156abd005e341d45783eb75cadcbfa05a6c9177545a0b1bb7504424a2ebf060b6559227a00115a9e53a93a6043928cd4b5380c69debd99cea2967d020f27b9d8ffde57de360e7344fa71dafc2f530c6934c49cd0686760fc3b2cb3da91e8d5c1edbd635c5e3d451d9707b3231e112c2d898e010e33ecbf6d55ee333c02b36e609b05edd3af9e5399488398d492ebb9ca250ba3d6f04d5a213a76d2ae55";
    pub(crate) const KEY: &str = "5dfec997ee8e4d93857d88567c97f89960e4dd457151a8514dbdc0e8c2102fe395a3ae59d073938282d6ed7d5f1e90841c4ee6b5bdce3c028b8c505a8efaa1604cb24d8f1eaf4b3fb153c6eabc5d436d02b3eb972d943f565b4d23fa1853692f2b38da408af3f13d91cdef8887743b91b039085300e4639800b87664941be76642eba468c2da31b6672a9763df0a0224d571296a5970de1671ed0ba856a763cfeaa52ace6a517e22e818ff2de43be75a27ff0d68d22e26489937085abb6b40a8b3ad933cd0ac1c492ecdf30269a65fc9c498dbc86d279f821c9fcf881ff88595865a1a0f0382deecda1f7b0eccf45fd3127fea583e10ff993d58af00e6609341";
    const FP: i64 = -3370862366631356593;

    fn exp(label: &[u8]) -> [u8; KEY_LEN] {
        let h = Sha256::digest(label);
        let mut out = [0u8; KEY_LEN];
        for c in out.chunks_mut(32) {
            c.copy_from_slice(&h);
        }
        out
    }

    fn rng() -> SeededEntropy {
        SeededEntropy::new(b"dh-tests")
    }

    #[test]
    fn rfc3526_valid_with_compatible_generators() {
        let p = rfc3526_p();
        // p mod 8 = 7, mod 3 = 2, mod 5 = 4, mod 24 = 23, mod 7 = 5 → todos los g valen.
        for g in [2, 3, 4, 5, 6, 7] {
            DhParams::validate(g, &p, 1, &mut rng()).unwrap_or_else(|e| panic!("g={g}: {e}"));
        }
    }

    #[test]
    fn telegram_prime_accepts_g3_rejects_incompatible_g() {
        let p = hex(TG_P);
        DhParams::validate(3, &p, 1, &mut rng()).expect("g=3 válido (p mod 3 = 2)");
        DhParams::validate(7, &p, 1, &mut rng()).expect("g=7 válido (p mod 7 = 6)");
        // p mod 8 = 3, p mod 5 = 3, p mod 24 = 11.
        for g in [2, 5, 6] {
            assert_eq!(
                DhParams::validate(g, &p, 1, &mut rng()).unwrap_err(),
                SecretError::InvalidDhConfig("g no genera el subgrupo de orden (p-1)/2"),
                "g={g}"
            );
        }
    }

    #[test]
    fn rejects_bad_primes_and_generators() {
        let p = rfc3526_p();
        assert!(DhParams::validate(1, &p, 1, &mut rng()).is_err());
        assert!(DhParams::validate(8, &p, 1, &mut rng()).is_err());
        assert!(DhParams::validate(-2, &p, 1, &mut rng()).is_err());
        assert!(DhParams::validate(2, &p[1..], 1, &mut rng()).is_err(), "255 bytes");
        let mut small = p.clone();
        small[0] = 0x7f;
        assert!(DhParams::validate(2, &small, 1, &mut rng()).is_err(), "bit alto a 0");
        // p - 2: impar, del mismo tamaño, compuesto o no seguro.
        let mut q = p.clone();
        q[KEY_LEN - 1] = 0xfd;
        assert_eq!(
            DhParams::validate(4, &q, 1, &mut rng()).unwrap_err(), // g = 4: sin condición de residuo
            SecretError::InvalidDhConfig("p no es un primo seguro")
        );
        // Otro impar de 2048 bits cercano: no es primo seguro (con probabilidad abrumadora).
        let mut not_safe = p.clone();
        not_safe[KEY_LEN - 2] ^= 0x10;
        assert_eq!(
            DhParams::validate(4, &not_safe, 1, &mut rng()).unwrap_err(),
            SecretError::InvalidDhConfig("p no es un primo seguro")
        );
    }

    #[test]
    fn miller_rabin_small_numbers() {
        let mut e = rng();
        for (n, prime) in [(2u32, true), (3, true), (4, false), (97, true), (561, false), (7919, true), (7917, false)] {
            assert_eq!(is_probable_prime(&BigUint::from(n), 16, &mut e).unwrap(), prime, "{n}");
        }
    }

    #[test]
    fn public_value_bounds() {
        let params = crate::testing::rfc3526_params();
        let p = BigUint::from_bytes_be(params.p());
        let bound: BigUint = BigUint::from(1u32) << (2048 - 64);
        let bad = [
            BigUint::from(0u32),
            BigUint::from(1u32),
            BigUint::from(2u32),
            &bound - 1u32,
            &p - &bound + 1u32,
            &p - 1u32,
            p.clone(),
        ];
        for v in bad {
            assert_eq!(params.check_public(&v.to_bytes_be()), Err(SecretError::InvalidPublicValue));
        }
        assert!(params.check_public(&bound.to_bytes_be()).is_ok());
        assert!(params.check_public(&(&p - &bound).to_bytes_be()).is_ok());
        assert!(params.check_public(&[]).is_err());
        assert!(params.check_public(&[1u8; 257]).is_err());
    }

    #[test]
    fn dh_matches_independent_vector() {
        let params = crate::testing::rfc3526_params();
        let a = DhSecret::from_exponent_bytes(&params, &exp(b"tc-secret a"));
        let b = DhSecret::from_exponent_bytes(&params, &exp(b"tc-secret b"));
        assert_eq!(a.public().to_vec(), hex(GA));
        assert_eq!(b.public().to_vec(), hex(GB));
        let k1 = a.shared_key(&params, b.public()).unwrap();
        let k2 = b.shared_key(&params, a.public()).unwrap();
        assert_eq!(k1.as_bytes().to_vec(), hex(KEY));
        assert_eq!(k2.as_bytes(), k1.as_bytes());
        assert_eq!(k1.fingerprint(), FP);
    }

    #[test]
    fn generate_mixes_server_random_and_rejects_bad_peer() {
        let params = crate::testing::rfc3526_params();
        let s1 = DhSecret::generate(&params, &mut SeededEntropy::new(b"x"), &[]).unwrap();
        let s2 = DhSecret::generate(&params, &mut SeededEntropy::new(b"x"), &[0xAA; 256]).unwrap();
        assert_ne!(s1.public(), s2.public(), "la aleatoriedad del servidor debe influir");
        assert!(params.check_public(s1.public()).is_ok());
        assert_eq!(s1.shared_key(&params, &[1]).unwrap_err(), SecretError::InvalidPublicValue);
        let p_minus_1 = (BigUint::from_bytes_be(params.p()) - 1u32).to_bytes_be();
        assert_eq!(s1.shared_key(&params, &p_minus_1).unwrap_err(), SecretError::InvalidPublicValue);
    }
}
