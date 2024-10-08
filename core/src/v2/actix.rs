#[cfg(feature = "actix-multipart")]
use super::schema::TypedData;
use super::{
    models::{
        DefaultOperationRaw, DefaultSchemaRaw, Either, Items, Parameter, ParameterIn, Response,
        SecurityScheme,
    },
    schema::{Apiv2Errors, Apiv2Operation, Apiv2Schema},
};

#[cfg(feature = "actix3")]
use actix_web::web::ReqData;

use actix_web::{
    http::StatusCode,
    web::{Bytes, Data, Form, Json, Path, Payload, Query},
    HttpRequest, HttpResponse, Responder,
};

use pin_project::pin_project;

use serde::Serialize;
#[cfg(feature = "serde_qs")]
use serde_qs::actix::QsQuery;

use std::{
    collections::BTreeMap,
    fmt,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

// Trait for modifying operations
pub trait OperationModifier: Apiv2Schema + Sized {
    fn update_parameter(_op: &mut DefaultOperationRaw) {}
    fn update_response(_op: &mut DefaultOperationRaw) {}
    fn update_definitions(map: &mut BTreeMap<String, DefaultSchemaRaw>) {
        update_definitions_from_schema_type::<Self>(map);
    }
    fn update_security(op: &mut DefaultOperationRaw) {
        update_security::<Self>(op);
    }
    fn update_security_definitions(map: &mut BTreeMap<String, SecurityScheme>) {
        update_security_definitions::<Self>(map);
    }
}

#[cfg(feature = "nightly")]
impl<T> OperationModifier for T
where
    T: Apiv2Schema,
{
    default fn update_parameter(_op: &mut DefaultOperationRaw) {}
    default fn update_response(_op: &mut DefaultOperationRaw) {}
    default fn update_definitions(map: &mut BTreeMap<String, DefaultSchemaRaw>) {
        update_definitions_from_schema_type::<Self>(map);
    }
    default fn update_security(op: &mut DefaultOperationRaw) {
        update_security::<Self>(op);
    }
    default fn update_security_definitions(map: &mut BTreeMap<String, SecurityScheme>) {
        update_security_definitions::<Self>(map);
    }
}

impl<T> OperationModifier for Option<T>
where
    T: OperationModifier,
{
    fn update_parameter(op: &mut DefaultOperationRaw) {
        T::update_parameter(op);
    }

    fn update_response(op: &mut DefaultOperationRaw) {
        T::update_response(op);
    }

    fn update_definitions(map: &mut BTreeMap<String, DefaultSchemaRaw>) {
        T::update_definitions(map);
    }

    fn update_security(op: &mut DefaultOperationRaw) {
        T::update_security(op);
    }

    fn update_security_definitions(map: &mut BTreeMap<String, SecurityScheme>) {
        T::update_security_definitions(map);
    }
}

#[cfg(feature = "nightly")]
impl<T, E> OperationModifier for Result<T, E>
where
    T: OperationModifier,
{
    default fn update_parameter(op: &mut DefaultOperationRaw) {
        T::update_parameter(op);
    }

    default fn update_response(op: &mut DefaultOperationRaw) {
        T::update_response(op);
    }

    default fn update_definitions(map: &mut BTreeMap<String, DefaultSchemaRaw>) {
        T::update_definitions(map);
    }

    default fn update_security_definitions(map: &mut BTreeMap<String, SecurityScheme>) {
        T::update_security_definitions(map);
    }
}

impl<T, E> OperationModifier for Result<T, E>
where
    T: OperationModifier,
    E: Apiv2Errors,
{
    fn update_parameter(op: &mut DefaultOperationRaw) {
        T::update_parameter(op);
    }

    fn update_response(op: &mut DefaultOperationRaw) {
        T::update_response(op);
        E::update_error_definitions(op);
    }

    fn update_definitions(map: &mut BTreeMap<String, DefaultSchemaRaw>) {
        T::update_definitions(map);
        E::update_definitions(map);
    }

    fn update_security_definitions(map: &mut BTreeMap<String, SecurityScheme>) {
        T::update_security_definitions(map);
    }
}

// Implement Apiv2Schema for various types
impl<T> Apiv2Schema for Data<T> {}
#[cfg(not(feature = "nightly"))]
impl<T> OperationModifier for Data<T> {}
#[cfg(feature = "actix3")]
impl<T: std::clone::Clone> Apiv2Schema for ReqData<T> {}
#[cfg(not(feature = "nightly"))]
#[cfg(feature = "actix3")]
impl<T: std::clone::Clone> OperationModifier for ReqData<T> {}

macro_rules! impl_empty {
    { $($ty:ty),+ } => {
        $(
            impl Apiv2Schema for $ty {}
            #[cfg(not(feature = "nightly"))]
            impl OperationModifier for $ty {}
        )+
    }
}

impl Apiv2Operation for HttpResponse {
    fn operation() -> DefaultOperationRaw {
        Default::default()
    }

    fn security_definitions() -> BTreeMap<String, SecurityScheme> {
        Default::default()
    }

    fn definitions() -> BTreeMap<String, DefaultSchemaRaw> {
        Default::default()
    }
}

impl_empty!(HttpRequest, HttpResponse, Bytes, Payload);

#[cfg(not(feature = "nightly"))]
mod manual_impl {
    use super::OperationModifier;

    impl<'a> OperationModifier for &'a str {}
    impl<'a, T: OperationModifier> OperationModifier for &'a [T] {}

    macro_rules! impl_simple {
        { $ty:ty } => {
            impl OperationModifier for $ty {}
        };
    }

    impl_simple!(char);
    impl_simple!(String);
    impl_simple!(bool);
    impl_simple!(f32);
    impl_simple!(f64);
    impl_simple!(i8);
    impl_simple!(i16);
    impl_simple!(i32);
    impl_simple!(u8);
    impl_simple!(u16);
    impl_simple!(u32);
    impl_simple!(i64);
    impl_simple!(i128);
    impl_simple!(isize);
    impl_simple!(u64);
    impl_simple!(u128);
    impl_simple!(usize);
    #[cfg(feature = "chrono")]
    impl_simple!(chrono::NaiveDateTime);
    #[cfg(feature = "rust_decimal")]
    impl_simple!(rust_decimal::Decimal);
    #[cfg(feature = "url")]
    impl_simple!(url::Url);
    #[cfg(feature = "uuid")]
    impl_simple!(uuid::Uuid);
}

#[cfg(feature = "chrono")]
impl<T: chrono::TimeZone> OperationModifier for chrono::DateTime<T> {}

// Implementations for other extractors
#[cfg(feature = "nightly")]
impl<T> Apiv2Schema for Json<T> {
    default const NAME: Option<&'static str> = None;
    default fn raw_schema() -> DefaultSchemaRaw {
        Default::default()
    }
}

impl<T: Apiv2Schema> Apiv2Schema for Json<T> {
    const NAME: Option<&'static str> = T::NAME;
    fn raw_schema() -> DefaultSchemaRaw {
        T::raw_schema()
    }
}

impl<T> OperationModifier for Json<T>
where
    T: Apiv2Schema,
{
    fn update_parameter(op: &mut DefaultOperationRaw) {
        op.parameters.push(Either::Right(Parameter {
            description: None,
            in_: ParameterIn::Body,
            name: "body".into(),
            required: true,
            schema: Some({
                let mut def = T::schema_with_ref();
                def.retain_ref();
                def
            }),
            ..Default::default()
        }));
    }

    fn update_response(op: &mut DefaultOperationRaw) {
        op.responses.insert(
            "200".into(),
            Either::Right(Response {
                description: Some("OK".into()),
                schema: Some({
                    let mut def = T::schema_with_ref();
                    def.retain_ref();
                    def
                }),
                ..Default::default()
            }),
        );
    }
}

#[cfg(feature = "actix-multipart")]
impl OperationModifier for actix_multipart::Multipart {
    fn update_parameter(op: &mut DefaultOperationRaw) {
        op.parameters.push(Either::Right(Parameter {
            description: None,
            in_: ParameterIn::FormData,
            name: "file_data".into(),
            required: true,
            data_type: Some(<actix_multipart::Multipart as TypedData>::data_type()),
            format: <actix_multipart::Multipart as TypedData>::format(),
            ..Default::default()
        }));
    }
}

#[cfg(feature = "actix-session")]
impl OperationModifier for actix_session::Session {
    fn update_definitions(_map: &mut BTreeMap<String, DefaultSchemaRaw>) {}
}

#[cfg(feature = "actix-files")]
impl OperationModifier for actix_files::NamedFile {
    fn update_definitions(_map: &mut BTreeMap<String, DefaultSchemaRaw>) {}
}

macro_rules! impl_param_extractor {
    { $ty:ty => $container:ident } => {
        #[cfg(feature = "nightly")]
        impl<T> Apiv2Schema for $ty {
            default const NAME: Option<&'static str> = None;
            default fn raw_schema() -> DefaultSchemaRaw {
                Default::default()
            }
        }

        impl<T> OperationModifier for $ty {
            fn update_parameter(op: &mut DefaultOperationRaw) {
                op.parameters.push(Either::Right(Parameter {
                    description: None,
                    in_: ParameterIn::$container,
                    name: stringify!($ty).to_lowercase().into(),
                    required: true,
                    ..Default::default()
                }));
            }
        }
    }
}

impl_param_extractor!(Path<T> => Path);
impl_param_extractor!(Query<T> => Query);
impl_param_extractor!(Form<T> => Form);
impl_param_extractor!(Data<T> => Data);

#[cfg(feature = "serde_qs")]
impl_param_extractor!(QsQuery<T> => Query);
