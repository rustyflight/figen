mod parser;

use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Expr, Lit, LitInt, Type};

#[derive(Debug)]
pub struct RegistryDefinition {
    version: LitInt,
    properties: Vec<PropertyDefinition>,
}

#[derive(Debug, PartialEq, Clone)]
enum PropertyDefinition {
    Struct(StructProperty),
    Scalar(ScalarProperty),
    Array(ArrayProperty),
}

#[derive(Debug, Clone)]
struct StructProperty {
    ident: Ident,
    ty: Type,
    fields: Vec<PropertyDefinition>,
}

#[derive(Debug, Clone)]
struct ScalarProperty {
    ident: Ident,
    ty: Type,
    default_value: Option<Expr>,
    optional: bool,
}

#[derive(Debug, Clone)]
struct ArrayProperty {
    ident: Ident,
    indices: Vec<(Lit, PropertyDefinition)>,
    elem: Box<PropertyDefinition>,
}

pub fn expand(input: RegistryDefinition) -> TokenStream {
    let expanded = input
        .properties
        .iter()
        .map(|prop| expand_property(prop, false));

    let defaults = input.properties.iter().map(|prop| {
        if let PropertyDefinition::Struct(s) = prop {
            Some(expand_default(s))
        } else {
            None
        }
    });

    quote!(
        #(#expanded)*

        #(#defaults)*
    )
    .into()
}

fn expand_default(prop: &StructProperty) -> TokenStream {
    let ty = &prop.ty;
    let fields = prop.fields.iter().map(|field| expand_field(field, false));

    quote!(
        impl Default for #ty {
            fn default() -> Self {
                Self {
                    #(#fields),*
                }
            }
        }
    )
}

fn expand_field(prop: &PropertyDefinition, as_array_element: bool) -> TokenStream {
    match prop {
        PropertyDefinition::Struct(s) => {
            let ident = &s.ident;
            let ty = &s.ty;
            let fields = s.fields.iter().map(|field| expand_field(field, false));

            let initializer = if prop.is_optional() && !prop.has_default_value() {
                quote!(None)
            } else {
                let token_stream = quote!(#ty { #(#fields),* });
                if prop.is_optional() {
                    quote!(Some(#token_stream))
                } else {
                    token_stream
                }
            };

            if as_array_element {
                initializer
            } else {
                quote!(
                    #ident: #initializer
                )
            }
        }
        PropertyDefinition::Array(a) => {
            let ident = &a.ident;
            let elems = a.indices.iter().map(|(_, def)| expand_field(def, true));
            quote!(
                #ident: [#(#elems),*]
            )
        }
        PropertyDefinition::Scalar(s) => {
            let ident = &s.ident;
            let default_value = s.default_value.as_ref();

            if as_array_element {
                return if prop.is_optional() {
                    if default_value.is_some() {
                        quote!(Some(#default_value))
                    } else {
                        quote!(None)
                    }
                } else {
                    quote!(#default_value)
                };
            }

            let default_value = if let Some(value) = default_value {
                quote!(#ident: #value.try_into().expect("Failed to convert default value to type"))
            } else {
                quote!(#ident: Default::default())
            };

            default_value
        }
    }
}

fn expand_property(prop: &PropertyDefinition, parent_is_optional: bool) -> TokenStream {
    match prop {
        PropertyDefinition::Struct(s) => {
            let ty = &s.ty;
            let fields = s.fields.iter().map(|field| {
                let ident = field.ident();
                let is_optional = field.is_optional() && !parent_is_optional;
                let ty = field.ty(is_optional);

                let mut property_attrs = vec![];
                if field.is_optional() || field.has_default_value() || parent_is_optional {
                    property_attrs.push(quote!(optional))
                };

                if let PropertyDefinition::Array(array) = field {
                    let indices: Vec<&Lit> = array.indices.iter().map(|(index, _)| index).collect();
                    property_attrs.push(quote!(indices = [#(#indices),*]));
                }

                quote!(
                    #[property(#(#property_attrs),*)]
                    pub #ident: #ty
                )
            });

            // Recursively expand nested structs and array element types
            let structs = s
                .fields
                .iter()
                .filter(|prop| {
                    if let PropertyDefinition::Scalar(_) = prop {
                        false
                    } else {
                        true
                    }
                })
                .map(|field| expand_property(field, parent_is_optional || field.is_optional()));

            quote!(
                #[derive(figen::Configuration)]
                pub struct #ty {
                    #(#fields),*
                }

                #(#structs)*
            )
        }
        PropertyDefinition::Array(a) => {
            match a.elem.as_ref() {
                PropertyDefinition::Scalar(_) => quote!(), // Ignore scalar elements, they will be expanded in the struct as fields
                PropertyDefinition::Struct(_) => {
                    expand_property(a.elem.as_ref(), parent_is_optional || a.elem.is_optional())
                }
                PropertyDefinition::Array(_) => {
                    expand_property(a.elem.as_ref(), parent_is_optional || a.elem.is_optional())
                }
            }
        }
        _ => panic!("Unsupported property type for expansion"),
    }
}

impl PropertyDefinition {
    fn merge(&mut self, other: PropertyDefinition) -> syn::Result<()> {
        match (self, other) {
            (PropertyDefinition::Struct(s1), PropertyDefinition::Struct(s2)) => {
                assert_eq!(
                    s1.ident, s2.ident,
                    "Cannot merge two structs with different identifiers"
                );
                for field in s2.fields {
                    if !s1.fields.contains(&field) {
                        s1.fields.push(field);
                    } else {
                        let existing_field = s1.fields.iter_mut().find(|f| f == &&field).unwrap();
                        existing_field.merge(field)?;
                    }
                }
                Ok(())
            }
            (PropertyDefinition::Array(a1), PropertyDefinition::Array(a2)) => {
                assert_eq!(
                    a1.ident, a2.ident,
                    "Cannot merge two arrays with different identifiers"
                );

                a1.elem.merge(*a2.elem)?;
                for index in a2.indices {
                    if !a1.indices.contains(&index) {
                        a1.indices.push(index);
                    } else {
                        let (_, prop) = a1.indices.iter_mut().find(|i| i == &&index).unwrap();
                        prop.merge(index.1)?;
                    }
                }
                Ok(())
            }
            (PropertyDefinition::Scalar(s1), PropertyDefinition::Scalar(s2)) => {
                assert_eq!(
                    s1.ident, s2.ident,
                    "Cannot merge two scalars with different identifiers"
                );
                if s1.ty != s2.ty {
                    return Err(syn::Error::new(
                        s1.ident.span(),
                        format!(
                            "Cannot merge scalars with different types: {:?} and {:?}",
                            s1.ty, s2.ty
                        ),
                    ));
                }
                if s1.optional != s2.optional {
                    return Err(syn::Error::new(
                        s1.ident.span(),
                        format!(
                            "Cannot merge scalars with different optionality: {:?} and {:?}",
                            s1.optional, s2.optional
                        ),
                    ));
                }
                Ok(())
            }
            (a, b) => Err(syn::Error::new(
                a.ident().span(),
                format!(
                    "Unable to merge unsupported property type combination, {:?} TO {:?}",
                    a, b
                ),
            )),
        }
    }

    fn ident(&self) -> &Ident {
        match self {
            PropertyDefinition::Struct(s) => &s.ident,
            PropertyDefinition::Scalar(s) => &s.ident,
            PropertyDefinition::Array(a) => &a.ident,
        }
    }

    fn ty(&self, optional: bool) -> Type {
        match self {
            PropertyDefinition::Scalar(s) => {
                let ty = s.ty.clone();
                if optional {
                    syn::parse2(quote!(Option<#ty>)).expect("Failed to parse optional type")
                } else {
                    ty
                }
            }
            PropertyDefinition::Struct(s) => {
                let ty = s.ty.clone();
                if optional {
                    syn::parse2(quote!(Option<#ty>)).expect("Failed to parse optional type")
                } else {
                    ty
                }
            }
            PropertyDefinition::Array(a) => {
                let ty = a.elem.ty(optional);
                let size = a.indices.len();
                syn::parse2(quote!([#ty; #size])).expect("Failed to parse array type")
            }
        }
    }

    fn is_optional(&self) -> bool {
        match self {
            PropertyDefinition::Scalar(s) => s.optional,
            PropertyDefinition::Struct(s) => s.fields.iter().all(|f| f.is_optional()),
            PropertyDefinition::Array(a) => a.elem.as_ref().is_optional(),
        }
    }

    fn has_default_value(&self) -> bool {
        match self {
            PropertyDefinition::Scalar(s) => s.default_value.is_some(),
            PropertyDefinition::Struct(s) => s.fields.iter().all(|f| f.has_default_value()),
            PropertyDefinition::Array(a) => a.indices.iter().all(|(_, f)| f.has_default_value()),
        }
    }
}

impl ScalarProperty {
    pub fn new(
        ident: Ident,
        ty: Type,
        default_value: Option<Expr>,
        optional: bool,
    ) -> Self {
        ScalarProperty {
            ident,
            ty,
            default_value,
            optional,
        }
    }
}

impl StructProperty {
    pub fn new(ident: Ident) -> Self {
        let ty_name_ident = Ident::new(
            stringcase::pascal_case(format!("{}Config", ident.to_string()).as_str()).as_str(),
            ident.span(),
        );
        let ty: Type = syn::parse2(quote!(#ty_name_ident)).expect("Failed to parse type");
        StructProperty {
            ident,
            ty,
            fields: vec![],
        }
    }

    pub fn add_field(&mut self, property: PropertyDefinition) {
        self.fields.push(property);
    }
}

impl ArrayProperty {
    pub fn new(ident: Ident, index: Lit, elem: PropertyDefinition) -> Self {
        ArrayProperty {
            ident,
            indices: vec![(index, elem.clone())],
            elem: Box::new(elem),
        }
    }
}

impl RegistryDefinition {
    fn new(version: LitInt) -> Self {
        RegistryDefinition {
            version,
            properties: vec![],
        }
    }

    fn push(&mut self, property: PropertyDefinition) -> syn::Result<()> {
        if !self.properties.contains(&property) {
            self.properties.push(property);
        } else {
            let prop = self
                .properties
                .iter_mut()
                .find(|p| *p == &property)
                .unwrap();
            prop.merge(property)?;
        }
        Ok(())
    }
}

impl PartialEq for StructProperty {
    fn eq(&self, other: &Self) -> bool {
        self.ident == other.ident
    }
}

impl PartialEq for ScalarProperty {
    fn eq(&self, other: &Self) -> bool {
        self.ident == other.ident
    }
}

impl PartialEq for ArrayProperty {
    fn eq(&self, other: &Self) -> bool {
        self.ident == other.ident
    }
}
