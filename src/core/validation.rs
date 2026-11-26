//! Input validation shared by the editors and command builders.
//!
//! Nothing here shells out: every value is validated before it can reach a
//! process argument list.

/// Parse a dotted-decimal IPv4 address, returning its four octets.
pub fn parse_ipv4(value: &str) -> Result<[u8; 4], String> {
    let parts: Vec<&str> = value.trim().split('.').collect();
    if parts.len() != 4 {
        return Err("Ingresa una direccion IPv4 valida.".to_string());
    }
    let mut octets = [0u8; 4];
    for (index, part) in parts.iter().enumerate() {
        if part.is_empty() || part.len() > 3 || !part.chars().all(|c| c.is_ascii_digit()) {
            return Err("Ingresa una direccion IPv4 valida.".to_string());
        }
        let octet: u32 = part
            .parse()
            .map_err(|_| "Ingresa una direccion IPv4 valida.".to_string())?;
        if octet > 255 {
            return Err("Cada octeto debe estar entre 0 y 255.".to_string());
        }
        octets[index] = octet as u8;
    }
    Ok(octets)
}

/// Normalise a Bluetooth address to `AA:BB:CC:DD:EE:FF`.
pub fn normalize_mac(value: &str) -> Result<String, String> {
    let candidate = value.trim().to_ascii_uppercase();
    let parts: Vec<&str> = candidate.split(':').collect();
    if parts.len() != 6 {
        return Err("MAC invalida (XX:XX:XX:XX:XX:XX)".to_string());
    }
    for part in &parts {
        if part.len() != 2 || !part.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err("MAC invalida (XX:XX:XX:XX:XX:XX)".to_string());
        }
    }
    Ok(parts.join(":"))
}

/// Parse a MAC address into its six bytes.
pub fn mac_bytes(value: &str) -> Result<[u8; 6], String> {
    let normalized = normalize_mac(value)?;
    let mut bytes = [0u8; 6];
    for (index, part) in normalized.split(':').enumerate() {
        bytes[index] = u8::from_str_radix(part, 16).map_err(|_| "MAC invalida".to_string())?;
    }
    Ok(bytes)
}

const MAX_HOSTNAME_LENGTH: usize = 253;
const MAX_LABEL_LENGTH: usize = 63;

/// Validate and lowercase a DNS hostname.
pub fn normalize_hostname(value: &str) -> Result<String, String> {
    let candidate = value.trim().to_ascii_lowercase();
    if candidate.is_empty() || candidate.len() > MAX_HOSTNAME_LENGTH {
        return Err("El host debe tener entre 1 y 253 caracteres.".to_string());
    }
    if candidate.starts_with('.') || candidate.ends_with('.') || candidate.contains("..") {
        return Err("Host no valido: usa letras, digitos, guiones y puntos.".to_string());
    }
    for label in candidate.split('.') {
        if label.is_empty() || label.len() > MAX_LABEL_LENGTH {
            return Err("Cada etiqueta del host puede tener hasta 63 caracteres.".to_string());
        }
        if label.starts_with('-') || label.ends_with('-') {
            return Err("Las etiquetas no pueden empezar ni acabar en guion.".to_string());
        }
        if !label
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err("Host no valido: usa letras, digitos, guiones y puntos.".to_string());
        }
    }
    Ok(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipv4_rejects_out_of_range_octets() {
        assert!(parse_ipv4("192.168.1.1").is_ok());
        assert!(parse_ipv4("256.1.1.1").is_err());
        assert!(parse_ipv4("1.2.3").is_err());
        assert!(parse_ipv4("1.2.3.x").is_err());
    }

    #[test]
    fn mac_is_normalised() {
        assert_eq!(
            normalize_mac("fc:fc:48:11:22:33").unwrap_or_default(),
            "FC:FC:48:11:22:33"
        );
        assert!(normalize_mac("FC:FC:48").is_err());
    }

    #[test]
    fn hostnames_are_lowercased_and_checked() {
        assert_eq!(
            normalize_hostname("Example.COM").unwrap_or_default(),
            "example.com"
        );
        assert!(normalize_hostname("-bad.example").is_err());
        assert!(normalize_hostname("bad..example").is_err());
    }
}
