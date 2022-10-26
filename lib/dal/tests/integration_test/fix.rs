use dal::{
    workflow_runner::workflow_runner_state::WorkflowRunnerStatus, ActionPrototype,
    ConfirmationPrototype, ConfirmationResolver, ConfirmationResolverId, DalContext, Fix, FixBatch,
    FixCompletionStatus, StandardModel, SystemId, WorkflowPrototypeId, WorkflowRunner,
};
use dal_test::helpers::component_payload::ComponentPayload;
use dal_test::{
    helpers::builtins::{Builtin, SchemaBuiltinsTestHarness},
    test,
};
use serde::Deserialize;

/// Expected output shape from running `skopeo inspect`, which is used by the builtin action
/// in this test module.
#[derive(Deserialize, Debug)]
struct SkopeoOutput {
    #[serde(rename = "Name")]
    name: String,
}

#[test]
async fn confirmation_to_action(ctx: &DalContext) {
    let (payload, _confirmation_resolver_id, action_workflow_prototype_id, _action_name) =
        setup_confirmation_resolver_and_get_action_prototype(ctx).await;

    let run_id = rand::random();
    let (_runner, runner_state, func_binding_return_values, _created_resources, _updated_resources) =
        WorkflowRunner::run(
            ctx,
            run_id,
            action_workflow_prototype_id,
            payload.component_id,
        )
        .await
        .expect("could not perform workflow runner run");
    assert_eq!(runner_state.status(), WorkflowRunnerStatus::Success);

    let mut maybe_skopeo_output_name: Option<String> = None;
    for func_binding_return_value in &func_binding_return_values {
        for stream in func_binding_return_value
            .get_output_stream(ctx)
            .await
            .expect("could not get output stream from func binding return value")
            .unwrap_or_default()
        {
            let maybe_skopeo_output: serde_json::Result<SkopeoOutput> =
                serde_json::from_str(&stream.message);
            if let Ok(skopeo_output) = maybe_skopeo_output {
                if maybe_skopeo_output_name.is_some() {
                    panic!(
                        "already found skopeo output with name: {:?}",
                        maybe_skopeo_output_name
                    );
                }
                maybe_skopeo_output_name = Some(skopeo_output.name);
            }
        }
    }
    let skopeo_outputname =
        maybe_skopeo_output_name.expect("could not find name via skopeo output");
    assert_eq!(skopeo_outputname, "docker.io/systeminit/whiskers");
}

#[test]
async fn confirmation_to_fix(ctx: &DalContext) {
    let (payload, confirmation_resolver_id, action_workflow_prototype_id, action_name) =
        setup_confirmation_resolver_and_get_action_prototype(ctx).await;

    // Create the batch.
    let mut batch = FixBatch::new(ctx, "toddhoward@systeminit.com")
        .await
        .expect("could not create fix execution batch");
    assert!(batch.started_at().is_none());
    assert!(batch.finished_at().is_none());
    assert!(batch.completion_status().is_none());

    // Create all fix(es) before starting the batch.
    let mut fix = Fix::new(
        ctx,
        *batch.id(),
        confirmation_resolver_id,
        payload.component_id,
    )
    .await
    .expect("could not create fix");
    assert!(fix.started_at().is_none());
    assert!(fix.finished_at().is_none());
    assert!(fix.completion_status().is_none());

    // NOTE(nick): batches are stamped as started inside their job.
    batch
        .stamp_started(ctx)
        .await
        .expect("could not stamp batch as started");
    assert!(batch.started_at().is_some());
    assert!(batch.finished_at().is_none());
    assert!(batch.completion_status().is_none());

    let run_id = rand::random();
    fix.run(ctx, run_id, action_workflow_prototype_id, action_name)
        .await
        .expect("could not run fix");
    assert!(fix.started_at().is_some());
    assert!(fix.finished_at().is_some());
    let completion_status = fix
        .completion_status()
        .expect("no completion status found for fix");
    assert_eq!(completion_status, &FixCompletionStatus::Success);

    // NOTE(nick): batches are stamped as finished inside their job.
    let batch_completion_status = batch
        .stamp_finished(ctx)
        .await
        .expect("could not complete batch");
    assert!(batch.finished_at().is_some());
    assert_eq!(
        batch
            .completion_status()
            .expect("no completion status for batch"),
        &FixCompletionStatus::Success
    );
    assert_eq!(batch_completion_status, FixCompletionStatus::Success);

    let found_batch = fix
        .fix_batch(ctx)
        .await
        .expect("could not get fix execution batch")
        .expect("no fix execution batch found");
    assert_eq!(batch.id(), found_batch.id());
}

async fn setup_confirmation_resolver_and_get_action_prototype(
    ctx: &DalContext,
) -> (
    ComponentPayload,
    ConfirmationResolverId,
    WorkflowPrototypeId,
    String,
) {
    let mut harness = SchemaBuiltinsTestHarness::new();
    let payload = harness
        .create_component(ctx, "systeminit/whiskers", Builtin::DockerImage)
        .await;

    let confirmation_prototype = ConfirmationPrototype::get_by_component_and_name(
        ctx,
        payload.component_id,
        "Has docker image resource?",
        payload.schema_id,
        payload.schema_variant_id,
        SystemId::NONE,
    )
    .await
    .expect("could not find confirmation prototype")
    .expect("no confirmation prototype found");

    let confirmation_resolver = confirmation_prototype
        .run(ctx, payload.component_id, SystemId::NONE)
        .await
        .expect("could not run confirmation prototype");

    let mut found_confirmation_resolvers = ConfirmationResolver::list(ctx)
        .await
        .expect("could not list confirmation resolvers");
    let found_confirmation_resolver = found_confirmation_resolvers
        .pop()
        .expect("found confirmation resolvers is empty");
    assert!(found_confirmation_resolvers.is_empty());
    assert_eq!(found_confirmation_resolver.id(), confirmation_resolver.id());

    let expected_action_name = "create";
    let mut filtered_action_prototypes = confirmation_resolver
        .recommended_actions(ctx)
        .await
        .expect("could not find recommended actions from confirmation resolver")
        .into_iter()
        .filter(|a| a.name() == expected_action_name)
        .collect::<Vec<ActionPrototype>>();
    let filtered_action_prototype = filtered_action_prototypes
        .pop()
        .expect("empty filtered action prototypes");
    assert!(filtered_action_prototypes.is_empty());
    assert_eq!(filtered_action_prototype.name(), expected_action_name);

    let found_action_prototype = ActionPrototype::find_by_name(
        ctx,
        expected_action_name,
        payload.schema_id,
        payload.schema_variant_id,
        SystemId::NONE,
    )
    .await
    .expect("could not find action prototype")
    .expect("no action prototype found");

    assert_eq!(found_action_prototype.id(), filtered_action_prototype.id());

    (
        payload,
        *found_confirmation_resolver.id(),
        found_action_prototype.workflow_prototype_id(),
        found_action_prototype.name().to_string(),
    )
}
