use crate::registry::{ArrayProperty, PropertyDefinition, ScalarProperty, StructProperty};
use crate::registry::{Attr, PropertyDefinitionRaw, RegistryDefinition};
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::parse::{Parse, ParseBuffer, ParseStream};
use syn::{Error, Expr, Lit, LitInt, LitStr, Token};

impl Parse for RegistryDefinition {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut registry_name: Option<Ident> = None;
        let mut version: Option<LitInt> = None;

        while !input.is_empty() {
            let fork = input.fork();
            let _ = fork.parse::<Ident>()?;
            if fork.parse::<Token![=]>().is_err() {
                break;
            }

            let header_key: Ident = input.parse()?;
            let _ = input.parse::<Token![=]>()?;

            match header_key.to_string().as_str() {
                "name" => {
                    if registry_name.is_some() {
                        return Err(Error::new(
                            header_key.span(),
                            "Duplicate `name` header in config_registry!",
                        ));
                    }
                    registry_name = Some(input.parse()?);
                }
                "version" => {
                    if version.is_some() {
                        return Err(Error::new(
                            header_key.span(),
                            "Duplicate `version` header in config_registry!",
                        ));
                    }
                    version = Some(input.parse()?);
                }
                _ => {
                    return Err(Error::new(
                        header_key.span(),
                        "Unknown config_registry! header; supported headers are `name` and `version`",
                    ));
                }
            }
        }

        let registry_name = registry_name.ok_or(Error::new(
            input.span(),
            "Missing required `name = <Ident>` header in config_registry!",
        ))?;
        let version = version.ok_or(Error::new(
            input.span(),
            "Missing required `version = <integer>` header in config_registry!",
        ))?;

        let mut registry_def = RegistryDefinition::new(registry_name.clone(), version);
        while !input.is_empty() {
            // Parse the property definition and append it to the registry
            let ParsedPropertyDefinition(prop_def, raw_def) =
                ParsedPropertyDefinition::parse_with_name(input, &registry_name)?;

            registry_def.push(prop_def)?;
            registry_def.registry_entries.push(raw_def);
        }

        Ok(registry_def)
    }
}

impl ParsedPropertyDefinition {
    fn parse_with_name(input: ParseStream, registry_name: &Ident) -> syn::Result<Self> {
        // Parse the raw definition as defined in the macro
        let ident: Ident = input.parse()?;
        let span = ident.span();

        let content: ParseBuffer;
        syn::parenthesized!(content in input);

        let key_lit = content.parse::<LitStr>()?;
        let key_span = key_lit.span();

        let mut raw_definition = PropertyDefinitionRaw::new(ident, key_lit);

        // Check for optional attributes
        let lookahead = content.lookahead1();
        if lookahead.peek(Token![,]) {
            while !content.is_empty() {
                let _ = content.parse::<Token![,]>()?;
                let ident = content.parse::<syn::Ident>()?;
                let token_eq = content.parse::<Token![=]>(); // Optional equals token

                let value = match token_eq {
                    Ok(_) => {
                        // If there is an equals token, parse the value
                        content.parse::<Expr>().ok()
                    }
                    Err(_) => {
                        // If there is no equals token, but ident is provided it is a boolean attribute
                        Some(Expr::Lit(syn::ExprLit {
                            attrs: vec![],
                            lit: Lit::Bool(syn::LitBool {
                                value: true,
                                span: ident.span(),
                            }),
                        }))
                    }
                };

                raw_definition.add_attr(Attr::new(ident, value));
            }
        }

        // Convert the raw definition into the final PropertyDefinition
        let key = raw_definition.key.value();
        let mut key_parts: Vec<&str> = key.split(".").collect();
        if key_parts.len() == 0 {
            return Err(Error::new(input.span(), "Property key cannot be empty"));
        }

        // Last part must always be a scalar type
        let key = key_parts.pop().ok_or(Error::new(
            input.span(),
            "Property key must have at least one part",
        ))?;
        let ty = raw_definition.get_final_type()?;
        let scalar = ScalarProperty::new(
            Ident::new(sanitize_key(key).as_str(), key_span),
            ty,
            raw_definition.get_attr("default"),
            raw_definition.get_attr("optional").is_some(),
        );
        let mut prop_def = PropertyDefinition::Scalar(scalar);

        // If the key is an array, we need to wrap the scalar in an Array definition
        if is_array_key(key) {
            prop_def = PropertyDefinition::Array(ArrayProperty::new(
                Ident::new(sanitize_key(key).as_str(), key_span),
                get_array_index(key)?,
                prop_def,
            ));
        }

        // Build the property definition structure for nested properties
        while !key_parts.is_empty() {
            let key = key_parts.pop().unwrap();
            let ident = Ident::new(sanitize_key(key).as_str(), span);

            let mut struct_prop = StructProperty::new(ident);
            struct_prop.add_field(prop_def);
            prop_def = PropertyDefinition::Struct(struct_prop);

            if is_array_key(key) {
                prop_def = PropertyDefinition::Array(ArrayProperty::new(
                    Ident::new(sanitize_key(key).as_str(), key_span),
                    get_array_index(key)?,
                    prop_def,
                ));
            }
        }

        // Create root Struct based on the macro-level registry name.
        let mut root = StructProperty::new_root(registry_name.clone());
        root.add_field(prop_def);

        Ok(ParsedPropertyDefinition(
            PropertyDefinition::Struct(root),
            raw_definition,
        ))
    }
}

fn get_array_index(key: &str) -> syn::Result<Lit> {
    // Extract the index from the key, assuming it is in the format "key[index]"
    if let Some(start) = key.find('[') {
        if let Some(end) = key.find(']') {
            let index_str = &key[start + 1..end];
            let index_lit = syn::parse2(quote!(#index_str))?;

            Ok(index_lit)
        } else {
            Err(Error::new(
                Span::call_site(),
                "Missing closing bracket for array index",
            ))
        }
    } else {
        Err(Error::new(Span::call_site(), "No array index found in key"))
    }
}

fn is_array_key(key: &str) -> bool {
    // Check if the key contains array brackets
    key.contains('[') && key.contains(']')
}

fn sanitize_key(key: &str) -> String {
    // Convert the key to snake_case and remove array brackets
    let sanitized = if let (Some(li), Some(_)) = (key.find('['), key.find(']')) {
        let (key, _) = key.split_at(li);
        key
    } else {
        key
    };
    stringcase::snake_case(sanitized)
}

struct ParsedPropertyDefinition(PropertyDefinition, PropertyDefinitionRaw);
