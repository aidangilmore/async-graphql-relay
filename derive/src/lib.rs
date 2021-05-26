use proc_macro::TokenStream;
use syn::{parse_macro_input, Data, DeriveInput, Ident};

#[macro_use]
extern crate quote;
extern crate proc_macro;

/// RelayGlobalID is used to create a scalar global ID type.
/// # Example
/// ```
/// #[derive(RelayGlobalID)]
/// pub struct ID(
///     pub String,
///     /// This type is generated by the macro #[derive(RelayNodeEnum)] and will be in same scope as it
///     pub SchemaNodeTypes,
/// );
/// 
/// // It can then be used on your GraphQL Objects
/// #[derive(SimpleObject)]
/// pub struct Tenant {
///     pub id: ID, // <- See how it uses the ID type defined above
///     pub name: String,
/// }
/// ```
#[proc_macro_derive(RelayGlobalID)]
pub fn derive_relay_global_id(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let m = quote! {
        impl From<&#name> for String {
            fn from(id: &#name) -> Self {
                let node_type = id.1.clone() as u32;
                let mut uuid = id.0.clone();
                if uuid.len() < 36 {
                    panic!("ID type must only contain a UUIDv4");
                }
                uuid.remove(8);
                uuid.remove(12);
                uuid.remove(16);
                uuid.remove(20);
                format!("{}{}", uuid, node_type)
            }
        }
        #[async_graphql::Scalar]
        impl async_graphql::ScalarType for #name {
            fn parse(_value: async_graphql::Value) -> async_graphql::InputValueResult<Self> {
                unimplemented!();
            }

            fn to_value(&self) -> async_graphql::Value {
                async_graphql::Value::String(String::from(self))
            }
        }
    };

    TokenStream::from(m)
}

/// RelayNodeEnum implements fetching of any object from its gloablly unqiue ID. This is required for the Relay `node` query which is used to refetch objects.
/// # Example
/// ```
/// #[derive(Interface, RelayNodeEnum)]
/// #[graphql(field(name = "id", type = "String"))]
/// pub enum Node {
///     User(User),
///     // Put all of your Object's in this enum
/// }
/// 
/// #[derive(SimpleObject)]
/// pub struct User {
///     pub id: ID,
///     pub name: String,
/// }
/// 
/// impl User {
///     // Then implement the `get` method on all of your Objects
///     pub async fn get(_ctx: RelayContext, id: String) -> Option<Node> {
///         // You can access global state such as a database using the RelayContext argument.
///         Some(
///             User {
///                 id: ID(id, SchemaNodeTypes::User),
///                 name: "Oscar".to_string(),
///             }
///             .into(),
///         )
///     }
/// }
/// 
/// // Finally implement the `node` query on your root query resolver
/// #[Object]
/// impl QueryRoot {
///     // This is the query you need to implement
///     async fn node(&self, id: String) -> Option<Node> {
///         Node::get(RelayContext::nil(), id).await
///     }
/// }
/// ```
#[proc_macro_derive(RelayNodeEnum)]
pub fn derive_relay_node(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;
    let variants = match input.data {
        Data::Enum(e) => e
            .variants
            .into_iter()
            .map(|v| v.ident)
            .collect::<Vec<Ident>>(),
        _ => {
            panic!("The RelayNode macro must be used on an enum type");
        }
    };
    let variant_node_type = (0..variants.len()).map(|v| (v + 1).to_string());

    let m = quote! {
        #[derive(Clone)]
        pub enum SchemaNodeTypes {
            Unknown = 0,
            #(
                #variants,
            )*
        }

        impl #name {
            pub async fn get(ctx: async_graphql_relay::RelayContext, relay_id: String) -> Option<Node> {
                if relay_id.len() < 32 {
                    None?
                }
                let (id, node_type) = relay_id.split_at(32);
                let mut id = id.to_string();
                id.insert(8, '-');
                id.insert(13, '-');
                id.insert(18, '-');
                id.insert(23, '-');

                match node_type {
                    #(
                        #variant_node_type => <#variants>::get(ctx, id.to_string()).await,
                    )*
                    _ => None
                }
            }
        }
    };

    TokenStream::from(m)
}
