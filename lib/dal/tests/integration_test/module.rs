use dal::module::Module;
use dal::pkg::export::PkgExporter;
use dal::{DalContext, Schema};
use dal_test::test;
use si_pkg::{SocketSpecArity, SocketSpecKind};

#[test]
async fn list_modules(ctx: &DalContext) {
    let modules = Module::list_installed(ctx)
        .await
        .expect("unable to get installed modules");

    let mut module_names: Vec<String> = modules.iter().map(|m| m.name().to_string()).collect();
    module_names.sort();

    assert_eq!(20, modules.len());

    let expected_installed_module_names = vec![
        "BadValidations".to_string(),
        "ValidatedInput".to_string(),
        "ValidatedOutput".to_string(),
        "dummy-secret".to_string(),
        "fallout".to_string(),
        "katy perry".to_string(),
        "large even lego".to_string(),
        "large odd lego".to_string(),
        "medium even lego".to_string(),
        "medium odd lego".to_string(),
        "pet_shop".to_string(),
        "pirate".to_string(),
        "si-aws-ec2-2023-09-26-2".to_string(),
        "si-coreos-2023-09-13".to_string(),
        "si-docker-image-2023-09-13".to_string(),
        "si-intrinsic-funcs".to_string(),
        "small even lego".to_string(),
        "small odd lego".to_string(),
        "starfield".to_string(),
        "swifty".to_string(),
    ];

    assert_eq!(expected_installed_module_names, module_names);
}

#[test]
async fn get_fallout_module(ctx: &DalContext) {
    let modules = Module::list_installed(ctx)
        .await
        .expect("unable to get installed modules");

    let mut filtered_modules: Vec<Module> = modules
        .into_iter()
        .filter(|m| m.name() == "fallout")
        .collect();

    assert_eq!(1, filtered_modules.len());

    if let Some(fallout_module) = filtered_modules.pop() {
        let associated_funcs = fallout_module
            .list_associated_funcs(ctx)
            .await
            .expect("Unable to get association funcs");
        let associated_schemas = fallout_module
            .list_associated_schemas(ctx)
            .await
            .expect("Unable to get association schemas");

        assert_eq!("fallout", fallout_module.name());
        assert_eq!("System Initiative", fallout_module.created_by_email());
        assert_eq!("2019-06-03", fallout_module.version());
        assert_eq!(3, associated_funcs.len());
        assert_eq!(1, associated_schemas.len());
    }
}

#[test]
async fn module_export_simple(ctx: &mut DalContext) {
    let schema = Schema::find_by_name(ctx, "dummy-secret")
        .await
        .expect("unable to get schema")
        .expect("schema not found");

    let default_schema_variant = schema
        .get_default_schema_variant(ctx)
        .await
        .expect("Unable to find the default schema variant id");

    assert!(default_schema_variant.is_some());

    let name = "Paul's Test Pkg".to_string();
    let description = "The Bison".to_string();
    let version = "2019-06-03".to_string();
    let user = "System Initiative".to_string();
    let mut exporter = PkgExporter::new_module_exporter(
        name.clone(),
        version.clone(),
        Some(description.clone()),
        &user,
        vec![schema.id()],
        ctx.get_workspace_default_change_set_id()
            .await
            .expect("unable to get default changeset id"),
    );

    let exported_pkg = exporter
        .export_as_spec(ctx)
        .await
        .expect("unable to get the pkg spec");

    assert_eq!(exported_pkg.name, name.clone());
    assert_eq!(exported_pkg.description, description.clone());
    assert_eq!(exported_pkg.version, version.clone());
    assert_eq!(exported_pkg.created_by, user.clone());
    assert_eq!(exported_pkg.funcs.len(), 13);

    let pkg_schemas = exported_pkg.clone().schemas;
    assert_eq!(pkg_schemas.len(), 1);

    let pkg_schema = pkg_schemas
        .first()
        .expect("unable to get the package schema");
    assert_eq!(pkg_schema.variants.len(), 1);

    let pkg_schema_spec = pkg_schema.clone().data.expect("unable to get schema spec");
    assert_eq!(pkg_schema_spec.name, "dummy-secret");
    assert_eq!(pkg_schema_spec.category, "test exclusive");

    let pkg_schema_variant = pkg_schema
        .variants
        .first()
        .expect("unable to get the schema variant");
    assert_eq!(pkg_schema_variant.name, "v0");
    assert_eq!(pkg_schema_variant.auth_funcs.len(), 1);
    assert_eq!(pkg_schema_variant.leaf_functions.len(), 1);
    assert_eq!(pkg_schema_variant.sockets.len(), 1);
    assert_eq!(pkg_schema_variant.si_prop_funcs.len(), 2);
    assert_eq!(pkg_schema_variant.root_prop_funcs.len(), 1);

    let socket = pkg_schema_variant
        .sockets
        .first()
        .expect("unable to get the socket");
    assert_eq!(socket.name, "dummy");
    assert_eq!(socket.inputs.len(), 1);

    let socket_spec = socket.clone().data.expect("unable to get socket spec data");
    assert_eq!(socket_spec.name, "dummy");
    assert_eq!(socket_spec.arity, SocketSpecArity::One);
    assert_eq!(socket_spec.kind, SocketSpecKind::Output);
    assert_eq!(
        socket_spec.connection_annotations,
        "[{\"tokens\":[\"dummy\"]}]"
    );

    let mut exported_pkg_func_names: Vec<String> =
        exported_pkg.funcs.iter().map(|f| f.name.clone()).collect();
    exported_pkg_func_names.sort();

    let expected_func_names = vec![
        "si:identity".to_string(),
        "si:resourcePayloadToValue".to_string(),
        "si:setArray".to_string(),
        "si:setBoolean".to_string(),
        "si:setInteger".to_string(),
        "si:setMap".to_string(),
        "si:setObject".to_string(),
        "si:setString".to_string(),
        "si:unset".to_string(),
        "si:validation".to_string(),
        "test:qualificationDummySecretStringIsTodd".to_string(),
        "test:scaffoldDummySecretAsset".to_string(),
        "test:setDummySecretString".to_string(),
    ];

    assert_eq!(exported_pkg_func_names, expected_func_names);
}