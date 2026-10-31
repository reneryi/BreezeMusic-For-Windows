use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use base64::{engine::general_purpose::STANDARD as BASE64_STD, Engine as _};
use md5::{Digest, Md5};
use num_bigint::BigUint;
use rand::Rng;
use rsa::{
    pkcs8::DecodePublicKey,
    traits::PublicKeyParts,
    Pkcs1v15Encrypt, RsaPublicKey,
};
use std::collections::BTreeMap;

type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;
type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;
type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;
type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

/// 酷狗概念版 (Lite) 专属协议常量 — 对齐 MakcRe/KuGouMusicApi 与 BreezeMusic Android KugouSigner.kt
pub const APP_ID: u32 = 3116;
pub const CLIENT_VERSION: u32 = 11440;
pub const ANDROID_SALT: &str = "LnT6xpN3khm36zse0QzvmgTZ3waWdRSA";
pub const WEB_SALT: &str = "NVPh5oo715z5DIWAeQlhMDsWXXQV4hwt";
pub const STANDARD_SALT: &str = "OIlwieks28dk2k092lksi2UIkp";
pub const TRACKER_KEY_SECRET: &str = "185672dd44712f60bb1736df5a377e82";

/// 概念版 (lite) RSA 1024-bit 公钥
pub const PUBLIC_KEY_LITE: &str = "MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQDECi0Np2UR87scwrvTr72L6oO01rBbbBPriSDFPxr3Z5syug0O24QyQO8bg27+0+4kBzTBTBOZ/WWU0WryL1JSXRTXLgFVxtzIY41Pe7lPOgsfTCn5kZcvKhYKJesKnnJDNr5/abvTGf+rHG3YRwsCHcQ08/q6ifSioBszvb3QiwIDAQAB";
/// 标准版 RSA 1024-bit 公钥
pub const PUBLIC_KEY_STANDARD: &str = "MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQDIAG7QOELSYoIJvTFJhMpe1s/gbjDJX51HBNnEl5HXqTW6lQ7LC8jr9fWZTwusknp+sVGzwd40MwP6U5yDE27M/X1+UR4tvOGOqp94TJtQ1EPnWGWXngpeIW5GxoQGao1rmYWAu6oi1z9XkChrsUdC6DJE5E221wf/4WLFxwAtRQIDAQAB";

#[derive(Debug, Clone)]
pub struct EncryptedPayload {
    pub ciphertext: String,
    pub key: String,
}

pub fn md5_hex(text: &str) -> String {
    md5_bytes_hex(text.as_bytes())
}

pub fn md5_bytes_hex(bytes: &[u8]) -> String {
    let mut hasher = Md5::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// 将 32 位十六进制 MD5 转为十进制大整数字符串 (用于计算酷狗设备 mid)
pub fn calc_mid(guid: &str) -> String {
    let hash_hex = md5_hex(guid);
    BigUint::parse_bytes(hash_hex.as_bytes(), 16)
        .map(|n| n.to_str_radix(10))
        .unwrap_or_else(|| "0".to_string())
}

pub fn random_alphanumeric(len: usize) -> String {
    const ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| {
            let idx = rng.gen_range(0..ALPHABET.len());
            ALPHABET[idx] as char
        })
        .collect()
}

pub fn random_hex(len: usize) -> String {
    const HEX_CHARS: &[u8] = b"0123456789abcdef";
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| {
            let idx = rng.gen_range(0..HEX_CHARS.len());
            HEX_CHARS[idx] as char
        })
        .collect()
}

/// 核心请求签名算法:
/// - Android (默认): MD5(ANDROID_SALT + key1=val1key2=val2... + body + ANDROID_SALT)
/// - Web: MD5(WEB_SALT + sorted("key=val") + body + WEB_SALT)
pub fn signature(params: &BTreeMap<String, String>, body: &str, web: bool) -> String {
    let salt = if web { WEB_SALT } else { ANDROID_SALT };
    let ordered = if web {
        let mut pairs: Vec<String> = params.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
        pairs.sort();
        pairs.join("")
    } else {
        params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("")
    };
    md5_hex(&format!("{}{}{}{}", salt, ordered, body, salt))
}

/// 登录与验证接口的 paramsKey: MD5(3116 + ANDROID_SALT + 11440 + time + extra)
pub fn params_key(time: i64, extra: &str) -> String {
    md5_hex(&format!(
        "{}{}{}{}{}",
        APP_ID, ANDROID_SALT, CLIENT_VERSION, time, extra
    ))
}

pub fn params_key_standard(time: i64, app_id: u32, client_ver: u32, extra: &str) -> String {
    md5_hex(&format!(
        "{}{}{}{}{}",
        app_id, STANDARD_SALT, client_ver, time, extra
    ))
}

/// 音源解析专用 trackerKey: MD5(hash_lower + "185672dd44712f60bb1736df5a377e82" + "3116" + mid + userid)
pub fn tracker_key(hash_lower: &str, mid: &str, user_id: &str) -> String {
    let uid = if user_id.trim().is_empty() {
        "0"
    } else {
        user_id.trim()
    };
    md5_hex(&format!(
        "{}{}{}{}{}",
        hash_lower.to_lowercase(),
        TRACKER_KEY_SECRET,
        APP_ID,
        mid,
        uid
    ))
}

/// AES-CBC PKCS7 加密（自动根据 key 字节长度适配 AES-128 或 AES-256），返回小写 Hex
pub fn aes_cbc_encrypt_hex(plaintext: &str, key: &str, iv: &str) -> Result<String, String> {
    let raw = aes_cbc_encrypt_bytes(plaintext.as_bytes(), key.as_bytes(), iv.as_bytes())?;
    Ok(hex::encode(raw))
}

pub fn aes_cbc_encrypt_bytes(data: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>, String> {
    if iv.len() != 16 {
        return Err(format!("Invalid AES IV length: {}", iv.len()));
    }
    match key.len() {
        16 => {
            let cipher = Aes128CbcEnc::new_from_slices(key, iv).map_err(|e| e.to_string())?;
            Ok(cipher.encrypt_padded_vec_mut::<Pkcs7>(data))
        }
        32 => {
            let cipher = Aes256CbcEnc::new_from_slices(key, iv).map_err(|e| e.to_string())?;
            Ok(cipher.encrypt_padded_vec_mut::<Pkcs7>(data))
        }
        other => Err(format!("Unsupported AES key length: {} bytes", other)),
    }
}

pub fn aes_cbc_decrypt_bytes(ciphertext: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>, String> {
    if iv.len() != 16 {
        return Err(format!("Invalid AES IV length: {}", iv.len()));
    }
    match key.len() {
        16 => {
            let cipher = Aes128CbcDec::new_from_slices(key, iv).map_err(|e| e.to_string())?;
            cipher
                .decrypt_padded_vec_mut::<Pkcs7>(ciphertext)
                .map_err(|e| format!("AES-128 decrypt error: {:?}", e))
        }
        32 => {
            let cipher = Aes256CbcDec::new_from_slices(key, iv).map_err(|e| e.to_string())?;
            cipher
                .decrypt_padded_vec_mut::<Pkcs7>(ciphertext)
                .map_err(|e| format!("AES-256 decrypt error: {:?}", e))
        }
        other => Err(format!("Unsupported AES key length: {} bytes", other)),
    }
}

/// 酷狗登录 payload 加密:
/// temporaryKey 为 16 位随机串，key = md5(temporaryKey) (32字节 ASCII = AES-256)，iv = key 后 16 字节
pub fn encrypt_login_params(text: &str) -> Result<EncryptedPayload, String> {
    let temp_key = random_alphanumeric(16);
    let key = md5_hex(&temp_key);
    let iv = &key[key.len() - 16..];
    let ciphertext = aes_cbc_encrypt_hex(text, &key, iv)?;
    Ok(EncryptedPayload {
        ciphertext,
        key: temp_key,
    })
}

/// 酷狗登录响应 secu_params 解密
pub fn decrypt_login_params(hex_cipher: &str, temp_key: &str) -> Result<String, String> {
    let key = md5_hex(temp_key);
    let iv = &key[key.len() - 16..];
    let cipher_bytes = hex::decode(hex_cipher.trim()).map_err(|e| e.to_string())?;
    let plain_bytes = aes_cbc_decrypt_bytes(&cipher_bytes, key.as_bytes(), iv.as_bytes())?;
    String::from_utf8(plain_bytes).map_err(|e| e.to_string())
}

/// 酷狗上游特色 RSA 加密（右侧零填充 + 原始模幂运算 raw modular exponentiation，非标准 PKCS#1）
/// 对齐 KugouSigner.rsa(text, useStandardKey)
pub fn rsa_raw_zero_pad_hex(text: &str) -> Result<String, String> {
    rsa_raw_zero_pad_with_key_hex(text, false)
}

pub fn rsa_raw_zero_pad_with_key_hex(text: &str, use_standard_key: bool) -> Result<String, String> {
    let pub_b64 = if use_standard_key {
        PUBLIC_KEY_STANDARD
    } else {
        PUBLIC_KEY_LITE
    };
    let der_bytes = BASE64_STD.decode(pub_b64).map_err(|e| e.to_string())?;
    let pub_key = RsaPublicKey::from_public_key_der(&der_bytes).map_err(|e| e.to_string())?;
    let modulus_bytes = (pub_key.n().bits() + 7) / 8;
    let raw = text.as_bytes();
    if raw.len() > modulus_bytes {
        return Err("RSA payload exceeds modulus length".to_string());
    }
    let mut padded = vec![0u8; modulus_bytes];
    padded[..raw.len()].copy_from_slice(raw);

    let m = BigUint::from_bytes_be(&padded);
    let n = BigUint::from_bytes_be(&pub_key.n().to_bytes_be());
    let e = BigUint::from_bytes_be(&pub_key.e().to_bytes_be());
    let c = m.modpow(&e, &n);
    let hex_str = c.to_str_radix(16);
    Ok(format!("{:0>width$}", hex_str, width = modulus_bytes * 2))
}

/// 标准 RSA/ECB/PKCS1Padding 加密（用于 risk/v2/r_register_dev 等接口）
pub fn rsa_pkcs1_hex(text: &str) -> Result<String, String> {
    let der_bytes = BASE64_STD.decode(PUBLIC_KEY_LITE).map_err(|e| e.to_string())?;
    let pub_key = RsaPublicKey::from_public_key_der(&der_bytes).map_err(|e| e.to_string())?;
    let mut rng = rand::thread_rng();
    let encrypted = pub_key
        .encrypt(&mut rng, Pkcs1v15Encrypt, text.as_bytes())
        .map_err(|e| e.to_string())?;
    Ok(hex::encode(encrypted))
}

/// 设备风控注册 (register_dev) 使用的 AES-128-CBC 加密
pub fn playlist_aes_encrypt(data: &str) -> Result<(String, String), String> {
    let mut rng = rand::thread_rng();
    let key: String = (0..6).map(|_| rng.gen_range(b'a'..=b'z') as char).collect();
    let md = md5_hex(&key);
    let enc_key = &md[0..16];
    let iv = &md[16..32];
    let encrypted = aes_cbc_encrypt_bytes(data.as_bytes(), enc_key.as_bytes(), iv.as_bytes())?;
    Ok((key, BASE64_STD.encode(encrypted)))
}

pub fn playlist_aes_decrypt(cipher_bytes: &[u8], key: &str) -> Result<String, String> {
    let md = md5_hex(key);
    let enc_key = &md[0..16];
    let iv = &md[16..32];
    let plain = aes_cbc_decrypt_bytes(cipher_bytes, enc_key.as_bytes(), iv.as_bytes())?;
    String::from_utf8(plain).map_err(|e| e.to_string())
}

/// 登录与 Token 刷新的 t1 令牌计算
pub fn calc_t1(previous: Option<&str>, now_ms: i64) -> String {
    let plain = format!("{}|{}", previous.unwrap_or(""), now_ms);
    aes_cbc_encrypt_hex(
        &plain,
        "5e4ef500e9597fe004bd09a46d8add98",
        "04bd09a46d8add98",
    )
    .unwrap_or_default()
}

/// 登录与 Token 刷新的 t2 设备指纹令牌计算
pub fn calc_t2(guid: &str, mac: &str, dev: &str, now_ms: i64) -> String {
    let plain = format!(
        "{}|0f607264fc6318a92b9e13c65db7cd3c|{}|{}|{}",
        guid, mac, dev, now_ms
    );
    aes_cbc_encrypt_hex(
        &plain,
        "fd14b35e3f81af3817a20ae7adae7020",
        "17a20ae7adae7020",
    )
    .unwrap_or_default()
}

/// Token 刷新 /v5/login_by_token 所需的 p3 加密字段
pub fn calc_p3(clienttime_sec: i64, token: &str) -> String {
    let plain = serde_json::json!({
        "clienttime": clienttime_sec,
        "token": token
    })
    .to_string();
    aes_cbc_encrypt_hex(
        &plain,
        "c24f74ca2820225badc01946dba4fdf7",
        "adc01946dba4fdf7",
    )
    .unwrap_or_default()
}

/// 标准化酷狗图片 URL（替换 {size} 为 400，规避缺少多级通配符证书的二级子域 HTTPS 报错）
pub fn normalize_image_url(raw: Option<&str>) -> String {
    let Some(s) = raw else { return String::new() };
    let trimmed = s.trim();
    if trimmed.is_empty() || trimmed.starts_with('{') || trimmed.starts_with('[') {
        return String::new();
    }
    let mut url = trimmed.replace("{size}", "400").replace("{param}", "400");
    if url.starts_with("//") {
        url = format!("https:{}", url);
    } else if url.to_lowercase().starts_with("http://") {
        let rest = &url[7..];
        let host = rest
            .split('/')
            .next()
            .unwrap_or("")
            .split(':')
            .next()
            .unwrap_or("")
            .to_lowercase();
        let lacks_multi_tls = host.ends_with(".bssdl.kugou.com")
            || host.ends_with(".bssdlbig.kugou.com")
            || (host.ends_with(".kugou.com")
                && host.matches('.').count() > 2
                && !host.ends_with(".cloud.kugou.com")
                && !host.ends_with(".tx.kugou.com"));
        if !lacks_multi_tls {
            url = format!("https://{}", rest);
        }
    }
    url
}
