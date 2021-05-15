use strong_xml::XmlRead;

#[derive(Debug, XmlRead, PartialEq)]
#[xml(tag = "ldml")]
pub struct LDML {
    #[xml(child = "identity")]
    pub identity: IdentityTag,
    #[xml(child = "collations")]
    pub collations: Collations,
}

#[derive(Debug, XmlRead, PartialEq)]
#[xml(tag = "identity")]
pub struct IdentityTag {
    #[xml(child = "version")]
    pub version: Version,
    #[xml(child = "language")]
    pub language: Language,
    #[xml(child = "territory")]
    pub territory: Option<Territory>,
}

#[derive(Debug, XmlRead, PartialEq)]
#[xml(tag = "version")]
pub struct Version {
    #[xml(attr = "number")]
    pub number: String,
}

#[derive(Debug, XmlRead, PartialEq)]
#[xml(tag = "language")]
pub struct Language {
    #[xml(attr = "type")]
    pub r#type: String,
}

#[derive(Debug, XmlRead, PartialEq)]
#[xml(tag = "territory")]
pub struct Territory {
    #[xml(attr = "type")]
    pub r#type: String,
}

#[derive(Debug, XmlRead, PartialEq)]
#[xml(tag = "collations")]
pub struct Collations {
    #[xml(child = "collation")]
    pub collation: Vec<Collation>,
}

#[derive(Debug, XmlRead, PartialEq)]
#[xml(tag = "collation")]
pub struct Collation {
    #[xml(attr = "type")]
    pub r#type: String,
    #[xml(flatten_text = "cr", cdata)]
    pub rules: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tailoring() {
        assert_eq!(
            LDML::from_str(
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
            LDML {
                identity: IdentityTag {
                    version: Version {
                        number: "$Revision$".into()
                    },
                    language: Language {
                        r#type: "af".into()
                    },
                    territory: None,
                },
                collations: Collations {
                    collation: vec![Collation {
                        r#type: "standard".into(),
                        rules: vec!["&N<<<ŉ".into()]
                    }]
                }
            }
        );
    }
}
