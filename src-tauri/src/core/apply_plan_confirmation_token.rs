use rusqlite::Connection;
use serde_json::json;

use crate::{
    core::{apply_plan_operation_preview, apply_plan_persistence, apply_plan_results},
    error::{AppError, AppResult},
    models::{
        ApplyPlanConfirmationTokenReceipt, ApplyPlanConfirmationTokenUseState,
        CreateApplyPlanRunLogRequest, IssueApplyPlanConfirmationTokenRequest, LibrarySettings,
        PersistedApplyPlanStatus, PreviewApplyPlanOperationsRequest,
    },
};

pub fn issue_apply_plan_confirmation_token(
    connection: &Connection,
    settings: &LibrarySettings,
    request: IssueApplyPlanConfirmationTokenRequest,
) -> AppResult<ApplyPlanConfirmationTokenReceipt> {
    let plan =
        apply_plan_persistence::get_apply_plan(connection, request.plan_id)?.ok_or_else(|| {
            AppError::Message(format!(
                "Saved ApplyPlan {} was not found.",
                request.plan_id
            ))
        })?;
    if plan.status == PersistedApplyPlanStatus::Cancelled {
        return Err(AppError::Message(
            "Cannot issue a confirmation token for a cancelled ApplyPlan.".to_owned(),
        ));
    }

    let operation_preview = apply_plan_operation_preview::preview_apply_plan_operations(
        connection,
        settings,
        PreviewApplyPlanOperationsRequest {
            plan_id: request.plan_id,
            expected_plan_hash: request.expected_plan_hash.clone(),
        },
    )?;

    let Some(plan_hash) = plan
        .plan_hash
        .clone()
        .filter(|value| !value.trim().is_empty())
    else {
        return Err(AppError::Message(
            "Cannot issue a confirmation token because the ApplyPlan hash is missing.".to_owned(),
        ));
    };

    if let Some(expected_operation_set_hash) = request
        .expected_operation_set_hash
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if expected_operation_set_hash != operation_preview.operation_set_hash {
            return Err(AppError::Message(
                "Expected operation-set hash did not match the current backend operation preview. Reload the saved plan before requesting confirmation.".to_owned(),
            ));
        }
    }

    if operation_preview.operations.is_empty() {
        return Err(AppError::Message(
            "Cannot issue a confirmation token without at least one current backend-owned operation candidate.".to_owned(),
        ));
    }

    let token = generate_token()?;
    let summary = json!({
        "kind": "confirmation_token_v1",
        "planHash": plan_hash,
        "operationSetHash": operation_preview.operation_set_hash,
        "operationSetHashVersion": operation_preview.operation_set_hash_version,
        "operationSetHashAlgorithm": operation_preview.operation_set_hash_algorithm,
        "allowedOperationCount": operation_preview.operations.len(),
        "readOnlyBoundary": {
            "canProceedToApply": false,
            "canExecute": false,
            "note": "Token issuance binds the current operation-set preview but does not unlock the Apply executor."
        }
    })
    .to_string();

    let run = apply_plan_results::create_apply_plan_run_log(
        connection,
        CreateApplyPlanRunLogRequest {
            apply_plan_id: request.plan_id,
            status: Some("draft_log".to_owned()),
            backup_strategy: Some("copy_backup_first".to_owned()),
            confirmation_token: Some(token.clone()),
            total_items: Some(operation_preview.summary.total_items),
            skipped_items: Some(operation_preview.summary.skipped_items),
            failed_items: Some(0),
            summary: Some(summary),
        },
    )?;

    let mut caveats = operation_preview.caveats;
    push_unique(
        &mut caveats,
        "Backend-issued confirmation token V1 created for this operation-set hash.",
    );
    push_unique(
        &mut caveats,
        "Confirmation token is single-use, but Apply executor is still locked; no files changed.",
    );
    push_unique(
        &mut caveats,
        "Apply executor is still locked. This token cannot move, copy, delete, quarantine, replace, or restore files.",
    );

    Ok(ApplyPlanConfirmationTokenReceipt {
        token,
        token_id: run.id,
        plan_id: request.plan_id,
        plan_hash,
        operation_set_hash: operation_preview.operation_set_hash,
        operation_set_hash_algorithm: operation_preview.operation_set_hash_algorithm,
        operation_set_hash_version: operation_preview.operation_set_hash_version,
        source_plan_kind: plan.source_plan_kind,
        allowed_operation_count: operation_preview.operations.len() as i64,
        single_use_state: ApplyPlanConfirmationTokenUseState::Unused,
        issued_at: run.created_at,
        expires_at: None,
        can_proceed_to_apply: false,
        can_execute: false,
        caveats,
    })
}

fn generate_token() -> AppResult<String> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(|error| {
        AppError::Message(format!("Could not issue confirmation token: {error}"))
    })?;
    Ok(format!("aptok_v1_{}", hex::encode(bytes)))
}

fn push_unique(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_owned());
    }
}
