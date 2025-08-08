use crate::registry::RegistryDefinition;
use crate::registry::{ArrayProperty, PropertyDefinition, ScalarProperty, StructProperty};
use proc_macro2::{Ident, Span};
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseBuffer, ParseStream};
use syn::{Error, Expr, Lit, LitInt, LitStr, Token};

impl Parse for RegistryDefinition {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let _: Ident = input.parse()?;
        let _ = input.parse::<Token![=]>()?;
        let version: LitInt = input.parse()?;

        let mut registry_def = RegistryDefinition::new(version);
        while !input.is_empty() {
            // Parse the property definition and append it to the registry
            let ParsedPropertyDefinition(prop_def, raw_def) = input.parse()?;

            registry_def.push(prop_def)?;
        }

        Ok(registry_def)
    }
}

impl Parse for ParsedPropertyDefinition {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Parse the raw definition as defined in the macro
        let ident: Ident = input.parse()?;
        let span = ident.span();

        let content: ParseBuffer;
        syn::parenthesized!(content in input);

        let key_lit = content.parse::<LitStr>()?;
        let key_span = key_lit.span();
        let _ = content.parse::<Token![,]>()?;
        let group = content.parse::<syn::Ident>()?;

        let mut raw_definition = PropertyDefinitionRaw::new(ident, key_lit, group);

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
            raw_definition.is_custom_property(),
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

        // Create root Struct based on property group so grouped scalar values can be defined
        let mut root = StructProperty::new(raw_definition.group.clone());
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

struct PropertyDefinitionRaw {
    ident: Ident,
    key: LitStr,
    group: Ident,
    attributes: Vec<Attr>,
}

impl PropertyDefinitionRaw {
    fn new(ident: Ident, key: LitStr, group: Ident) -> Self {
        PropertyDefinitionRaw {
            ident,
            key,
            group,
            attributes: vec![],
        }
    }

    fn get_final_type(&self) -> syn::Result<syn::Type> {
        match self.ident.to_string().as_str() {
            "str_property" => {
                #[cfg(feature = "std")]
                {
                    Ok(syn::parse_quote!(String))
                }
                #[cfg(not(feature = "std"))]
                {
                    let max_len = self.get_attr("max_len").ok_or(Error::new(
                        self.ident.span(),
                        "Missing max_len attribute for nostd String property",
                    ))?;
                    Ok(syn::parse_quote!(heapless::String<#max_len>))
                }
            }
            "bool_property" => Ok(syn::parse_quote!(bool)),
            "num_property" => {
                let ty = self
                    .get_attr("ty")
                    .map(|ty| ty.into_token_stream())
                    .unwrap_or(quote!(i32));
                Ok(syn::parse_quote!(#ty))
            }
            "custom_property" => {
                let ty = self.get_attr("ty").ok_or(Error::new(
                    self.ident.span(),
                    "Missing type attribute for custom property",
                ))?;
                Ok(syn::parse_quote!(#ty))
            }
            &_ => Err(Error::new(
                self.ident.span(),
                format!("Unknown property definition: {}", self.ident),
            )),
        }
    }

    fn is_custom_property(&self) -> bool {
        self.ident.to_string().as_str() == "custom_property"
    }

    fn add_attr(&mut self, attr: Attr) {
        self.attributes.push(attr);
    }

    fn get_attr(&self, attr: &str) -> Option<Expr> {
        let result = self
            .attributes
            .iter()
            .find(|a| a.ident.to_string().as_str() == attr);
        result?.value.clone()
    }
}

struct Attr {
    ident: Ident,
    value: Option<Expr>,
}

impl Attr {
    fn new(ident: Ident, value: Option<Expr>) -> Self {
        Attr { ident, value }
    }
}
