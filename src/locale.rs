use strong_xml::XmlRead;
use crate::ldml::{LDML, Collation};
use std::convert::TryFrom;

// A more sensible format for the tailoring
#[derive(Debug, PartialEq)]
struct Locale {
    identity: Identity,
    collations: Vec<Collation>,
}

#[derive(Debug, PartialEq)]
struct Identity {
    version: String,
    language: String,
    territory: Option<String>,
}

impl From<LDML> for Locale {
    fn from(ldml: LDML) -> Self {
        Self {
            identity: Identity {
                version: ldml.identity.version.number,
                language: ldml.identity.language.r#type,
                territory: ldml.identity.territory.map(|t| t.r#type),
            },
            collations: ldml.collations.collation,
        }
    }
}

impl TryFrom<&str> for Locale {
    type Error = strong_xml::XmlError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Ok(Self::from(LDML::from_str(s)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ldml::*;

    #[test]
    fn test_tailoring() {
        assert_eq!(
            Locale::try_from(
                "<ldml>
                    <identity>
                        <version number=\"$Revision$\"/>
                        <language type=\"af\"/>
                    </identity>
                    <collations >
                        <collation type=\"standard\">
                            <cr><![CDATA[&N<<<ŉ]]></cr>
                        </collation>
                    </collations>
                </ldml>",
            ).unwrap(),
            Locale {
                identity: Identity {
                    version: "$Revision$".into(),
                    language: "af".into(),
                    territory: None,
                },
                collations: vec![Collation {
                    r#type: "standard".into(),
                    rules: vec!["&N<<<ŉ".into()]
                }]
            }
        )
    }
}
