//! LacisID calculation helpers
//!
//! LPG calculates candidate lacisIDs but does NOT issue/register them.
//! Registration authority belongs to mobes2.0 lacisIdService.
//!
//! Format: [prefix][productType(3)][MAC(12)][productCode(4)] = 20 digits
//! - NetworkDevice prefix: "4"  → total 20 chars
//! - araneaDevice prefix: "3"  → total 20 chars

/// Normalize MAC address: uppercase, no separators (e.g. "AA:BB:CC:DD:EE:FF" → "AABBCCDDEEFF")
pub fn normalize_mac_for_lacis_id(mac: &str) -> String {
    mac.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_uppercase()
}

/// NetworkDevice lacisID: [prefix=4][productType(3)][MAC(12)][productCode(4)] = 20 chars
pub fn compute_network_device_lacis_id(
    product_type: &str,
    mac: &str,
    product_code: &str,
) -> String {
    let normalized_mac = normalize_mac_for_lacis_id(mac);
    format!("4{}{}{}", product_type, normalized_mac, product_code)
}

/// araneaDevice lacisID: [prefix=3][productType(3)][MAC(12)][productCode(4)] = 20 chars
pub fn compute_aranea_device_lacis_id(product_type: &str, mac: &str, product_code: &str) -> String {
    let normalized_mac = normalize_mac_for_lacis_id(mac);
    format!("3{}{}{}", product_type, normalized_mac, product_code)
}

/// Default product code for network device types
pub fn default_product_code(network_device_type: &str) -> &str {
    match network_device_type {
        "Router" => "0000",
        "Switch" => "0000",
        "AccessPoint" => "0000",
        "Controller" => "0000",
        _ => "0000",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_mac() {
        assert_eq!(
            normalize_mac_for_lacis_id("AA:BB:CC:DD:EE:FF"),
            "AABBCCDDEEFF"
        );
        assert_eq!(
            normalize_mac_for_lacis_id("aa-bb-cc-dd-ee-ff"),
            "AABBCCDDEEFF"
        );
        assert_eq!(normalize_mac_for_lacis_id("AABBCCDDEEFF"), "AABBCCDDEEFF");
        assert_eq!(
            normalize_mac_for_lacis_id("aa:bb:cc:dd:ee:ff"),
            "AABBCCDDEEFF"
        );
    }

    #[test]
    fn test_network_device_lacis_id() {
        let id = compute_network_device_lacis_id("101", "AA:BB:CC:DD:EE:FF", "0000");
        assert_eq!(id, "4101AABBCCDDEEFF0000");
        assert_eq!(id.len(), 20);
    }

    #[test]
    fn test_aranea_device_lacis_id() {
        let id = compute_aranea_device_lacis_id("201", "11:22:33:44:55:66", "0001");
        assert_eq!(id, "3201112233445566_0001".replace("_", ""));
        // Correct: "32011122334455660001"
        let id2 = compute_aranea_device_lacis_id("201", "11:22:33:44:55:66", "0001");
        assert_eq!(id2, "32011122334455660001");
        assert_eq!(id2.len(), 20);
    }

    #[test]
    fn test_default_product_code() {
        assert_eq!(default_product_code("Router"), "0000");
        assert_eq!(default_product_code("Switch"), "0000");
        assert_eq!(default_product_code("AccessPoint"), "0000");
        assert_eq!(default_product_code("Unknown"), "0000");
    }
}
