//! This module contains [`create_func`] and everything it needs.

use base64::engine::general_purpose;
use base64::Engine;

use crate::func::authoring::{
    CreateFuncOptions, CreatedFunc, FuncAuthoringError, FuncAuthoringResult,
};
use crate::func::FuncKind;
use crate::{
    generate_name, DalContext, Func, FuncBackendKind, FuncBackendResponseType, SchemaVariant,
};

static DEFAULT_CODE_HANDLER: &str = "main";
static DEFAULT_ATTRIBUTE_CODE: &str = include_str!("data/defaults/attribute.ts");
static DEFAULT_CODE_GENERATION_CODE: &str = include_str!("data/defaults/code_generation.ts");
static DEFAULT_QUALIFICATION_CODE: &str = include_str!("data/defaults/qualification.ts");
static DEFAULT_ACTION_CODE: &str = include_str!("data/defaults/action.ts");
static DEFAULT_AUTHENTICATION_CODE: &str = include_str!("data/defaults/authentication.ts");

#[allow(dead_code)]
static DEFAULT_VALIDATION_CODE: &str = include_str!("data/defaults/validation.ts");

pub(crate) async fn create_func(
    ctx: &DalContext,
    kind: FuncKind,
    name: Option<String>,
    options: Option<CreateFuncOptions>,
) -> FuncAuthoringResult<CreatedFunc> {
    let func = match kind {
        FuncKind::Action => create_action_func(ctx, name, options).await?,
        FuncKind::Attribute => {
            create_attribute_func(ctx, name, FuncKind::Attribute, options).await?
        }
        FuncKind::Authentication => create_authentication_func(ctx, name, options).await?,
        FuncKind::CodeGeneration => {
            create_attribute_func(ctx, name, FuncKind::CodeGeneration, options).await?
        }
        FuncKind::Qualification => {
            create_attribute_func(ctx, name, FuncKind::Qualification, options).await?
        }
        kind => return Err(FuncAuthoringError::InvalidFuncKindForCreation(kind)),
    };

    Ok(CreatedFunc {
        id: func.id,
        handler: func.handler.as_ref().map(|h| h.to_owned()),
        kind: func.kind,
        name: func.name.to_owned(),
        code: func.code_plaintext()?,
    })
}

async fn create_func_stub(
    ctx: &DalContext,
    name: Option<String>,
    backend_kind: FuncBackendKind,
    backend_response_type: FuncBackendResponseType,
    code: &str,
    handler: &str,
) -> FuncAuthoringResult<Func> {
    let name = name.unwrap_or(generate_name());
    if Func::find_by_name(ctx, &name).await?.is_some() {
        return Err(FuncAuthoringError::FuncNameExists(name));
    }

    let code_base64 = general_purpose::STANDARD_NO_PAD.encode(code);

    let func = Func::new(
        ctx,
        name,
        None::<String>,
        None::<String>,
        None::<String>,
        false,
        false,
        backend_kind,
        backend_response_type,
        Some(handler),
        Some(code_base64),
    )
    .await?;

    Ok(func)
}

async fn create_action_func(
    ctx: &DalContext,
    name: Option<String>,
    _options: Option<CreateFuncOptions>,
) -> FuncAuthoringResult<Func> {
    let func = create_func_stub(
        ctx,
        name,
        FuncBackendKind::JsAction,
        FuncBackendResponseType::Action,
        DEFAULT_ACTION_CODE,
        DEFAULT_CODE_HANDLER,
    )
    .await?;

    //    if let Some(CreateFuncOptions::ActionOptions {
    //        schema_variant_id,
    //        action_kind,
    //    }) = options
    //    {
    //        ActionPrototype::new(
    //            ctx,
    //            *func.id(),
    //            action_kind,
    //            ActionPrototypeContext { schema_variant_id },
    //        )
    //        .await?;
    //    }

    Ok(func)
}

//async fn create_leaf_prototype(
//    ctx: &DalContext,
//    func: &Func,
//    schema_variant_id: SchemaVariantId,
//    variant: FuncVariant,
//) -> FuncResult<()> {
//    let leaf_kind = match variant {
//        FuncVariant::CodeGeneration => LeafKind::CodeGeneration,
//        FuncVariant::Qualification => LeafKind::Qualification,
//        _ => return Err(FuncError::FuncOptionsAndVariantMismatch),
//    };
//
//    let input_locations = match leaf_kind {
//        LeafKind::CodeGeneration => vec![LeafInputLocation::Domain],
//        LeafKind::Qualification => vec![LeafInputLocation::Domain, LeafInputLocation::Code],
//    };
//
//    SchemaVariant::upsert_leaf_function(
//        ctx,
//        schema_variant_id,
//        None,
//        leaf_kind,
//        &input_locations,
//        func,
//    )
//    .await?;
//
//    Ok(())
//}

async fn create_attribute_func(
    ctx: &DalContext,
    name: Option<String>,
    kind: FuncKind,
    _options: Option<CreateFuncOptions>,
) -> FuncAuthoringResult<Func> {
    let (code, handler, backend_kind, backend_response_type) = match kind {
        FuncKind::Attribute => (
            DEFAULT_ATTRIBUTE_CODE,
            DEFAULT_CODE_HANDLER,
            FuncBackendKind::JsAttribute,
            FuncBackendResponseType::Unset,
        ),
        FuncKind::CodeGeneration => (
            DEFAULT_CODE_GENERATION_CODE,
            DEFAULT_CODE_HANDLER,
            FuncBackendKind::JsAttribute,
            FuncBackendResponseType::CodeGeneration,
        ),
        FuncKind::Qualification => (
            DEFAULT_QUALIFICATION_CODE,
            DEFAULT_CODE_HANDLER,
            FuncBackendKind::JsAttribute,
            FuncBackendResponseType::Qualification,
        ),
        _ => {
            return Err(FuncAuthoringError::UnexpectedFuncKindCreatingAttributeFunc(
                kind,
            ));
        }
    };

    let func = create_func_stub(
        ctx,
        name,
        backend_kind,
        backend_response_type,
        code,
        handler,
    )
    .await?;

    // if let Some(options) = options {
    //     match (variant, options) {
    //         (
    //             FuncVariant::Attribute,
    //             CreateFuncOptions::AttributeOptions {
    //                 output_location, ..
    //             },
    //         ) => {
    //             // XXX: we need to search *up* the attribute tree to ensure that
    //             // the parent of this prop is not also set by a function. But we
    //             // should also hide props on the frontend if they are the
    //             // children of a value that is set by a function.
    //             let mut context_builder = AttributeContextBuilder::new();
    //             match output_location {
    //                 AttributeOutputLocation::OutputSocket {
    //                     external_provider_id,
    //                 } => {
    //                     context_builder.set_external_provider_id(external_provider_id);
    //                 }
    //                 AttributeOutputLocation::Prop { prop_id } => {
    //                     context_builder.set_prop_id(prop_id);
    //                 }
    //             }

    //             let context = context_builder.to_context()?;
    //             let mut prototype =
    //                 AttributePrototype::find_for_context_and_key(ctx, context, &None)
    //                     .await?
    //                     .pop()
    //                     .ok_or(FuncError::AttributePrototypeMissing)?;

    //             if let Some(func) = Func::get_by_id(ctx, &prototype.func_id()).await? {
    //                 if !func.is_intrinsic() {
    //                     return Err(FuncError::AttributePrototypeAlreadySetByFunc(
    //                         func.name().into(),
    //                     ));
    //                 }
    //             }

    //             prototype.set_func_id(ctx, *func.id()).await?;
    //         }
    //         (
    //             FuncVariant::CodeGeneration,
    //             CreateFuncOptions::CodeGenerationOptions { schema_variant_id },
    //         ) => {
    //             create_leaf_prototype(ctx, &func, schema_variant_id, variant).await?;
    //         }
    //         (
    //             FuncVariant::Qualification,
    //             CreateFuncOptions::QualificationOptions { schema_variant_id },
    //         ) => {
    //             create_leaf_prototype(ctx, &func, schema_variant_id, variant).await?;
    //         }
    //         (_, _) => return Err(FuncError::FuncOptionsAndVariantMismatch),
    //     }
    // }

    Ok(func)
}

async fn create_authentication_func(
    ctx: &DalContext,
    name: Option<String>,
    options: Option<CreateFuncOptions>,
) -> FuncAuthoringResult<Func> {
    let func = create_func_stub(
        ctx,
        name,
        FuncBackendKind::JsAuthentication,
        FuncBackendResponseType::Void,
        DEFAULT_AUTHENTICATION_CODE,
        DEFAULT_CODE_HANDLER,
    )
    .await?;

    if let Some(CreateFuncOptions::AuthenticationOptions { schema_variant_id }) = options {
        SchemaVariant::new_authentication_prototype(ctx, func.id, schema_variant_id).await?;
    }

    Ok(func)
}