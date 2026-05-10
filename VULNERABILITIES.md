# Backend Security Vulnerabilities

## 🔴 Critical (1)

| # | Vulnerability | File | Lines |
|---|---|---|---|
| 1 | **Auth bypass in debug mode** — `assert_login` skips ALL auth when `cfg!(debug_assertions)` is true, regardless of env vars. Any debug build is completely unprotected. | `authenticate.rs` | 200–207 |

## 🟠 High (5)

| # | Vulnerability | File | Lines |
|---|---|---|---|
| 2 | **Weak password hashing (SHA-1)** — Passwords verified with SHA-1 + simple salt. No bcrypt/argon2/scrypt stretching. | `authenticate.rs` | 295–304 |
| 3 | **SSRF via VLAN URL** — `vlan_url` from K-Net API response is fetched server-side with Basic Auth creds, leaking credentials if a user record is compromised. | `authenticate.rs` | 337–345 |
| 4 | **No CORS configuration** — No CORS layer applied despite importing `tower-http` CORS. | `main.rs` | 329–350 |
| 6 | **`.env` with credentials in working tree** — API credentials present on disk; accidental commit risk. | `.env` | — |
| 14 | **Inconsistent auth bypass logic** — `assert_login` uses `||` (always bypasses in debug) while `authenticate_user` uses `&&` (requires env var AND debug). Likely a bug. | `authenticate.rs` | 200 vs 232 |

## 🟡 Medium (6)

| # | Vulnerability | File | Lines |
|---|---|---|---|
| 5 | **No CSRF protection** — No CSRF tokens. Partially mitigated by `SameSite::Strict`. | `main.rs` | 40–142 |
| 7 | **Username enumeration** — Distinct error messages for "not found", "no password", and "wrong password". | `authenticate.rs` | 253–308 |
| 8 | **No global rate limiting** — Per-username only, in-memory, resets on restart. No IP-based or global limit. | `authenticate.rs` | 243–251 |
| 10 | **Unauthenticated PII exposure** — `/api/book/events` exposes all bookings (usernames + room numbers) without auth. | `main.rs` | 284–289 |
| 11 | **No Secure flag on cookie in debug** — Cookie sent over HTTP in debug builds. | `authenticate.rs` | 172–173 |
| 12 | **Missing HttpOnly flag** — Session cookie readable by JavaScript (XSS → session theft). | `authenticate.rs` | 167–178 |

## 🟢 Low (6)

| # | Vulnerability | File | Lines |
|---|---|---|---|
| 13 | **Internal error messages leaked** — `e.to_string()` returned to clients. | `main.rs` | 54–56 |
| 15 | **No input length validation** — No max length on username/password fields. | `authenticate.rs` | 226–230 |
| 16 | **Predictable booking IDs** — Random `u32` (small keyspace), though ownership checks exist. | `booker.rs` | 425 |
| 17 | **No TLS / binds 0.0.0.0** — Plaintext if no reverse proxy. | `main.rs` | 361 |
| 18 | **URL injection in K-Net API** — Username interpolated without URL encoding. | `authenticate.rs` | 254–256 |
| 19 | **Docker runs as root** — No `USER` directive in Dockerfile. | `Dockerfile` | — |

---

**Total: 18 vulnerabilities** (1 Critical, 5 High, 6 Medium, 6 Low)
