mod registry;

use darling::{FromAttributes, FromMeta};
use proc_macro::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
use syn::{parse_macro_input, Data, Ident, LitStr, Type};

#[derive(Debug, FromAttributes)]
#[darling(attributes(property))]
struct PropertyArgs {
    key: Option<LitStr>,
    #[darling(default)]
    optional: bool,
    #[darling(default)]
    flatten: bool,
    indices: Option<Vec<LitStr>>,
    array_ref: Option<ArrayRefArgs>,
}

#[derive(Debug, FromMeta)]
struct ArrayRefArgs {
    key: LitStr,
    prefix: Option<LitStr>,
}

struct FieldDefinition {
    pub ident: Ident,
    pub ty: Type,
    pub attrs: PropertyArgs,
}

#[proc_macro]
pub fn expand_config_registry(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as registry::RegistryDefinition);
    registry::expand(input)
        .into()
}


#[proc_macro_derive(Configuration, attributes(property))]
pub fn derive_configuration(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let ident = &input.ident;

    let data = match input.data {
        Data::Struct(d) => d,
        _ => {
            return syn::Error::new(input.span(), "Configuration can only be derived for structs")
                .to_compile_error()
                .into();
        }
    };

    let field_defs: Vec<FieldDefinition> = data.fields.iter().map(|field| {
        let attrs = &field.attrs;
        let has_property_attr = attrs.iter().any(|attr| {
            attr.path().is_ident("property")
        });
        assert!(has_property_attr, "Field {} does not have a #[property] attribute", field.ident.as_ref().unwrap());

        let attrs: PropertyArgs = PropertyArgs::from_attributes(&attrs).unwrap();

        FieldDefinition {
            ident: field.ident.clone().expect("Fields must have a name"),
            ty: field.ty.clone(),
            attrs,
        }
    }).collect();

    let bindings = field_defs.iter().map(|field_def| {
        let field = field_def.ident.clone();
        let key = field_def.attrs.key.clone().unwrap_or_else(|| LitStr::new(&field.to_string().as_str(), field.span()));

        let handle_err = if field_def.attrs.optional {
            quote!(
                Ok(r) => {
                    result = result.or(Ok(r));
                },
                Err(e) => {
                    if e != figen::error::Error::NotFound {
                        return Err(e);
                    }
                }
            )
        } else {
            quote!(
                Ok(r) => {
                    result = result.or(Ok(r));
                },
                Err(figen::error::Error::NotFound) => {
                    // If the field is required, we return a required error
                    return Err(figen::error::Error::Required);
                }
                Err(e) => {
                    return Err(e);
                }
            )
        };


        let bind_call = if let Type::Array(_) = &field_def.ty {
            let indices = field_def.attrs.indices.as_ref().map(|indices| {
                // Expand the indices to an array
                quote!(
                    {
                        let _indices = &[#(#indices),*];
                        assert!(_indices.len() == self.#field.len(), "Array indices length does not match array size");
                        figen::binder::ArrayConfigIndicesMode::Custom(_indices)
                    }
                )
            }).or_else(|| {
                Some(quote!(figen::binder::ArrayConfigIndicesMode::ZeroIndexed))
            });

            quote!(
                {

                    let mut _binder = figen::binder::ArrayConfigBinder::new(#indices, &mut self.#field);
                    match _binder.bind(path, loader) {
                        Ok(r) => {
                            result = result.or(Ok(r));
                        },
                        Err(e) => {
                            if e != figen::error::Error::NotFound {
                                return Err(e);
                            }
                        }
                    };
                }
            )
        } else {
            if let Some(array_ref) = &field_def.attrs.array_ref {
                let array_ref_key = &array_ref.key;
                let array_ref_prefix = &array_ref.prefix;

                let binder = if array_ref_prefix.is_some() {
                    quote!(
                        figen::binder::ArrayRefBinder::new(
                            #array_ref_key,
                            Some(#array_ref_prefix),
                            &mut self.#field
                        );
                    )
                } else {
                    quote!(
                        figen::binder::ArrayRefBinder::new(
                            #array_ref_key,
                            None,
                            &mut self.#field
                        );
                    )
                };

                quote!(
                    {
                        let mut _binder = #binder
                        match _binder.bind(path, loader) {
                            Ok(r) => {
                                result = result.or(Ok(r));
                            },
                            Err(e) => {
                                if e != figen::error::Error::NotFound {
                                    return Err(e);
                                }
                            }
                        };
                    }
                )
            } else {
                quote!(
                    match self.#field.bind(path, loader) {
                        #handle_err
                    };
                )
            }
        };

        // If the field is flattened, we bind it directly, otherwise we push the key to the path
        if field_def.attrs.flatten {
            bind_call
        } else {
            quote!(
                path.push(#key);
                #bind_call
                path.pop();
            )
        }
    });

    let result_expand = if field_defs.iter().all(|f| f.attrs.optional) {
        quote!(let mut result = Err(figen::error::Error::NotFound);)
    } else {
        quote!(let mut result = Err(figen::error::Error::Required);)
    };
    quote!(
        impl<T, U> figen::binder::ConfigBinder<T, U> for #ident where
            T: figen::BindPath,
            U: figen::loader::PropertyLoader,
        {
            fn bind(&mut self, path: &mut T, loader: &U) -> figen::error::Result<()> {
                #result_expand
                #(#bindings);*
                result
            }
        }
    ).into()
}
