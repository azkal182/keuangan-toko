use unicode_casefold::UnicodeCaseFold;

#[derive(Debug, PartialEq, Eq)]
pub struct CustomerIdentity {
    pub name: String,
    pub code: Option<String>,
    pub normalized_code: Option<String>,
    pub address: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct EmptyCustomerName;

impl CustomerIdentity {
    pub fn new(
        name: &str,
        code: Option<&str>,
        address: Option<&str>,
    ) -> Result<Self, EmptyCustomerName> {
        let name = name.trim();
        if name.is_empty() {
            return Err(EmptyCustomerName);
        }

        let code = optional_text(code);
        let normalized_code = code.as_deref().map(fold_customer_code);
        Ok(Self {
            name: name.to_owned(),
            code,
            normalized_code,
            address: optional_text(address),
        })
    }
}

pub fn fold_customer_code(code: &str) -> String {
    code.case_fold().collect()
}

pub fn fold_customer_name(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .case_fold()
        .collect()
}

fn optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_identity_and_case_folds_codes() {
        let customer = CustomerIdentity::new(" Contoh ", Some(" Straße "), None).unwrap();
        assert_eq!(customer.name, "Contoh");
        assert_eq!(customer.code.as_deref(), Some("Straße"));
        assert_eq!(customer.normalized_code.as_deref(), Some("strasse"));
    }

    #[test]
    fn rejects_blank_names_and_clears_blank_optional_fields() {
        assert_eq!(
            CustomerIdentity::new(" \u{2003}", None, None),
            Err(EmptyCustomerName)
        );
        let customer = CustomerIdentity::new("Contoh", Some(" "), Some(" ")).unwrap();
        assert_eq!(customer.code, None);
        assert_eq!(customer.normalized_code, None);
        assert_eq!(customer.address, None);
    }
}
