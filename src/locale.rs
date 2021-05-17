use crate::{
    collation_rules::{self, Collation},
    ldml::LDML,
};
use std::convert::TryFrom;
use strong_xml::XmlRead;

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

#[derive(Debug)]
enum Error {
    RuleParseError,
    XMLError,
}

impl TryFrom<LDML> for Locale {
    type Error = Error;
    fn try_from(ldml: LDML) -> Result<Self, Self::Error> {
        Ok(Self {
            identity: Identity {
                version: ldml.identity.version.number,
                language: ldml.identity.language.r#type,
                territory: ldml.identity.territory.map(|t| t.r#type),
            },
            collations: ldml
                .collations
                .collation
                .into_iter()
                .map(|c| {
                    Ok(Collation {
                        r#type: c.r#type,
                        rules: collation_rules::cldr(&c.rules.join(""))
                            .map_err(|_| Error::RuleParseError)?,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

impl TryFrom<&str> for Locale {
    type Error = Error;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::try_from(LDML::from_str(s).map_err(|_| Error::XMLError)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collation_rules::{CollationRules, Rule};

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
            )
            .unwrap(),
            Locale {
                identity: Identity {
                    version: "$Revision$".into(),
                    language: "af".into(),
                    territory: None,
                },
                collations: vec![Collation {
                    r#type: "standard".into(),
                    rules: CollationRules {
                        settings: vec![],
                        rules: vec![
                            Rule::SetContext {
                                sequence: "N".into(),
                                before: None,
                            },
                            Rule::Increment {
                                level: 3,
                                sequence: "ŉ".into(),
                                prefix: None,
                                extension: None,

                            }
                        ],
                    },
                }]
            }
        )
    }
}
