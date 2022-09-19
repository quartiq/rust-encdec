

use darling::FromMeta;
use proc_macro::{TokenStream};

use quote::{quote};
use syn::{parse_macro_input, DeriveInput, Data, NestedMeta, Meta, Lit};

use crate::attrs::{FieldAttrs, StructAttrs};

/// Encode derive helper
pub fn derive_encode_impl(input: TokenStream) -> TokenStream {

    let DeriveInput { ident, data, generics, attrs, .. } = parse_macro_input!(input);

    // Extract struct fields
    let s = match data {
        Data::Struct(s) => s,
        _ => panic!("Unsupported object type for derivation"),
    };

    // Parse struct attributes
    let struct_attrs = StructAttrs::parse(attrs.iter());

    // Fetch bounds for generics
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Build parser for each field
    let mut encoders = quote!{};
    let mut lengths = quote!{};

    s.fields.iter().enumerate().for_each(|(i, f)| {

        // Parse field attributes
        let attrs = FieldAttrs::parse(f.attrs.iter());

        // Generate field identifier
        let id = match f.ident.clone() {
            Some(id) => quote!{ #id },
            None => {
                let id = syn::Index::from(i);
                quote!{ #id }
            },
        };

        let ty = &f.ty;

        let call_encode = match (attrs.encode, attrs.length_of) {
            // Encode method override
            (Some(e), _) => quote!{
                index += #e(&self.#id, &mut buff[index..])?;
            },
            // Normal fields using normal encode
            (_, None) => quote!{ 
                index += self.#id.encode(&mut buff[index..])?;
            },
            // `length_of` types filled using length of target field
            (_, Some(v)) => quote!{ 
                let n = self.#v.encode_len()?;
                index += (n as #ty).encode(&mut buff[index..])?;
            },
        };

        let call_len = match attrs.encode_len {
            // Encode length override
            Some(l) => quote!{ index += #l(&self.#id)?; },
            // Default encode length method
            None => quote!{ index += self.#id.encode_len()?; },
        };

        encoders.extend(call_encode);
        lengths.extend(call_len);
    });

    // Override error return type if specified
    let err = match struct_attrs.error {
        Some(e) => quote!(#e),
        None => quote!(::encdec::Error),
    };

    quote! {
        impl #impl_generics ::encdec::Encode for #ident #ty_generics #where_clause {

            type Error = #err;

            fn encode_len(&self) -> Result<usize, Self::Error> {
                use ::encdec::Encode;

                let mut index = 0;
                
                #lengths

                Ok(index)
            }
            
            fn encode(&self, buff: &mut [u8]) -> Result<usize, Self::Error> {
                use ::encdec::Encode;

                let mut index = 0;
                
                #encoders

                Ok(index)
            }
        }
    }.into()
}
