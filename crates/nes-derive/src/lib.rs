use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{Data, DeriveInput, Field, Fields, Ident, parse_macro_input, spanned::Spanned};

#[proc_macro_derive(SaveState, attributes(save))]
pub fn derive_save_state(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;
    let container_name = quote::format_ident!("{}Data", name);

    let save_data = save_data(&container_name, &input.data);
    let save_impl = save_impl(&name, &container_name, &input.data);

    let tt = quote! {
        #save_data

        #save_impl
    };

    tt.into()
}

fn save_data(container_name: &Ident, data: &Data) -> TokenStream {
    match data {
        Data::Struct(data) => match data.fields {
            Fields::Named(ref fields) => {
                let fields = fields.named.iter().filter_map(|f| {
                    let name = &f.ident;
                    let ty = &f.ty;
                    let array = matches!(ty, syn::Type::Array(_));
                    let span = f.span();
                    match DataField::parse(f) {
                        DataField::Nested => {
                            Some(quote_spanned!(span=> #name: <#ty as SaveState>::Data ))
                        }
                        DataField::Normal if array => {
                            Some(quote_spanned!(span=> #[serde(with = "serde_arrays")] #name: #ty ))
                        }
                        DataField::Normal => Some(quote_spanned!(span=> #name: #ty )),
                        DataField::Skip => None,
                    }
                });

                quote! {
                    #[derive(::std::clone::Clone, ::serde::Serialize, ::serde::Deserialize)]
                    pub struct #container_name {
                        #(#fields),*
                    }
                }
            }
            Fields::Unnamed(ref fields) => {
                let fields = fields.unnamed.iter().filter_map(|f| {
                    let ty = &f.ty;
                    let span = f.span();
                    match DataField::parse(f) {
                        DataField::Nested => Some(quote_spanned!(span=> <#ty as SaveState>::Data)),
                        DataField::Normal => Some(quote_spanned!(span=> #ty)),
                        DataField::Skip => None,
                    }
                });

                quote! {
                    #[derive(::std::clone::Clone, ::serde::Serialize, ::serde::Deserialize)]
                    pub struct #container_name(
                        #(#fields),*
                    );
                }
            }
            Fields::Unit => quote! {
                #[derive(::std::clone::Clone, ::serde::Serialize, ::serde::Deserialize)]
                pub struct #container_name;
            },
        },
        _ => {
            let parent_span = container_name.span();
            quote_spanned! {parent_span=> const _:() = compile_error!("SaveState can only be derived by structs");}
        }
    }
}

fn save_impl(name: &Ident, container_name: &Ident, data: &Data) -> TokenStream {
    let (save_fields, restore_fields) = match data {
        Data::Struct(data) => match data.fields {
            Fields::Named(ref fields) => {
                let (save, restore) = fields
                    .named
                    .iter()
                    .filter_map(|f| {
                        let name = &f.ident;
                        match DataField::parse(f) {
                            DataField::Normal => Some((
                                quote! { #name: self.#name.clone() },
                                quote! { self.#name = state.#name.clone() },
                            )),
                            DataField::Nested => Some((
                                quote! { #name: self.#name.save_state() },
                                quote! { self.#name.restore_state(&state.#name) },
                            )),
                            DataField::Skip => None,
                        }
                    })
                    .collect::<(Vec<_>, Vec<_>)>();

                (
                    quote! { #container_name { #(#save),*} },
                    quote! { #(#restore);* },
                )
            }
            Fields::Unnamed(ref fields) => {
                let (save, restore) = fields
                    .unnamed
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, f)| {
                        let idx = syn::Index::from(idx);
                        match DataField::parse(f) {
                            DataField::Normal => Some((
                                quote! { self.#idx.clone() },
                                quote! { self.#idx = state.#idx.clone() },
                            )),
                            DataField::Nested => Some((
                                quote! { self.#idx.save_state() },
                                quote! { self.#idx.restore_state(&state.#idx) },
                            )),
                            DataField::Skip => None,
                        }
                    })
                    .collect::<(Vec<_>, Vec<_>)>();
                (
                    quote! {#container_name(#(#save),*)},
                    quote! {#(#restore);* },
                )
            }
            Fields::Unit => (quote! {#container_name}, quote! {}),
        },
        _ => return quote! {},
    };
    quote! {
        impl ::nes_traits::SaveState for #name {
            type Data = #container_name;

            fn save_state(&self) -> Self::Data {
                #save_fields
            }

            fn restore_state(&mut self, state: &Self::Data) {
                #restore_fields
            }
        }

    }
}

enum DataField {
    Nested,
    Normal,
    Skip,
}

impl DataField {
    fn parse(field: &Field) -> DataField {
        let mut skipped = false;
        let mut nested = false;
        for attr in field.attrs.iter() {
            if !attr.path().is_ident("save") {
                continue;
            }

            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("skip") {
                    skipped = true;
                }

                if meta.path.is_ident("nested") {
                    nested = true;
                }

                Ok(())
            });
        }

        if skipped {
            DataField::Skip
        } else if nested {
            DataField::Nested
        } else {
            DataField::Normal
        }
    }
}
