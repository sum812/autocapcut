/// validator.rs — Verify JWT offline + xác định LicenseStatus.

use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

/// Public key của license server được embed vào binary.
/// Thay PLACEHOLDER bằng public key RS256 thực khi generate keypair.
/// Generate: openssl genrsa -out private.pem 2048 && openssl rsa -in private.pem -pubout -out public.pem
const JWT_PUBLIC_KEY: &str = "-----BEGIN PUBLIC KEY-----\n\
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEApDvyCJ1aDP1dTPa9wEF2\n\
K/SA/o3xxF+6lw+lgLyVsfyRitjvuwnDIw3M9EUCGjkc3tsh0ufSTKynKdfsp2N0\n\
Opu0/K5kf3hzHnbAPWpl0QzisS2lELp3k3Zw2r4bwxlLB4rVPXYg7EEkGFkkGfDu\n\
wJZTgRnUR1BP1rF+aKLxxGDL3/c0cZzTXmjGC+lQm+WiC7dpKesGfUpV/3rQx5Bn\n\
PCSKKNLGZ05YkBO2bAinwu/2p+U0T1Twi+cWwpIwniTAHJ57nr2+L91jzfNMWWnt\n\
82gwchXL48nLWRvPu8at/M3td2LH57bhd/58/1AGizf6pp/irXPV6e2Sh5RZolvU\n\
TQIDAQAB\n\
-----END PUBLIC KEY-----\n";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub iss: String,
    pub sub: String,  // "license:{uuid}"
    pub iat: i64,
    pub exp: i64,
    pub fingerprint: String,
    pub plan: String,
    pub license_expires_at: Option<String>,
    pub max_projects: Option<u32>,
    pub features: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status")]
pub enum LicenseStatus {
    /// Token hợp lệ, không expire
    Valid {
        plan: String,
        license_expires_at: Option<String>,
        max_projects: Option<u32>,
        features: Vec<String>,
        days_until_token_expire: i64,
    },
    /// Token đã expire, đang trong grace period (3 ngày offline tolerance)
    GracePeriod {
        expires_in_hours: i64,
    },
    /// License đã hết hạn (business expiry, không phải token expiry)
    Expired {
        expired_at: String,
    },
    /// Chưa activate
    NotActivated,
    /// Token không hợp lệ (tampered, fingerprint mismatch, v.v.)
    Invalid {
        reason: String,
    },
}

impl LicenseStatus {
    pub fn is_usable(&self) -> bool {
        matches!(self, LicenseStatus::Valid { .. } | LicenseStatus::GracePeriod { .. })
    }

    pub fn max_projects(&self) -> Option<u32> {
        match self {
            LicenseStatus::Valid { max_projects, .. } => *max_projects,
            LicenseStatus::GracePeriod { .. } => None, // Grace = unlimited
            _ => Some(3), // Free tier
        }
    }
}

/// Verify JWT token với embedded public key (OFFLINE).
pub fn verify_jwt(token: &str) -> Result<JwtClaims, String> {
    if JWT_PUBLIC_KEY.contains("PLACEHOLDER") {
        return Err("JWT_PUBLIC_KEY chưa được cấu hình".into());
    }

    let decoding_key = DecodingKey::from_rsa_pem(JWT_PUBLIC_KEY.as_bytes())
        .map_err(|e| format!("Lỗi load public key: {}", e))?;

    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&["api.aicoachtools.com"]);

    decode::<JwtClaims>(token, &decoding_key, &validation)
        .map(|data| data.claims)
        .map_err(|e| format!("JWT verify thất bại: {}", e))
}

/// Verify JWT cho phép token đã expire (dùng khi check grace period).
pub fn verify_jwt_allow_expired(token: &str) -> Result<JwtClaims, String> {
    if JWT_PUBLIC_KEY.contains("PLACEHOLDER") {
        return Err("JWT_PUBLIC_KEY chưa được cấu hình".into());
    }

    let decoding_key = DecodingKey::from_rsa_pem(JWT_PUBLIC_KEY.as_bytes())
        .map_err(|e| format!("Lỗi load public key: {}", e))?;

    let mut validation = Validation::new(Algorithm::RS256);
    validation.validate_exp = false;
    validation.set_issuer(&["api.aicoachtools.com"]);

    decode::<JwtClaims>(token, &decoding_key, &validation)
        .map(|data| data.claims)
        .map_err(|e| format!("JWT decode thất bại: {}", e))
}

/// Tính toán LicenseStatus từ JWT token đã có trong cache.
pub fn compute_status(token: &str, current_fingerprint_id: &str) -> LicenseStatus {
    let now = chrono::Utc::now().timestamp();

    match verify_jwt(token) {
        Ok(claims) => {
            // Verify fingerprint match
            if claims.fingerprint != current_fingerprint_id {
                return LicenseStatus::Invalid {
                    reason: "Phần cứng thay đổi quá nhiều. Vui lòng kích hoạt lại.".into(),
                };
            }

            // Kiểm tra business expiry (license_expires_at)
            if let Some(ref exp_str) = claims.license_expires_at {
                if let Ok(exp_dt) = chrono::DateTime::parse_from_rfc3339(exp_str) {
                    if exp_dt.timestamp() < now {
                        return LicenseStatus::Expired {
                            expired_at: exp_str.clone(),
                        };
                    }
                }
            }

            let days_left = (claims.exp - now) / 86400;
            LicenseStatus::Valid {
                plan: claims.plan,
                license_expires_at: claims.license_expires_at,
                max_projects: claims.max_projects,
                features: claims.features,
                days_until_token_expire: days_left,
            }
        }
        Err(_) => {
            // Token có thể đã expire — thử với allow_expired để check grace period
            match verify_jwt_allow_expired(token) {
                Ok(claims) => {
                    // Grace period: 3 ngày sau khi token expire
                    let grace_end = claims.exp + 3 * 86400;
                    if now < grace_end {
                        LicenseStatus::GracePeriod {
                            expires_in_hours: (grace_end - now) / 3600,
                        }
                    } else {
                        LicenseStatus::Invalid {
                            reason: "Token hết hạn và đã qua grace period. Vui lòng kết nối internet để gia hạn.".into(),
                        }
                    }
                }
                Err(e) => LicenseStatus::Invalid { reason: e },
            }
        }
    }
}
