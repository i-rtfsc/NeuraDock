use tauri::State;

use crate::application::dtos::{
    CodexPaymentIntervalDto, CodexPaymentLinkDto, CodexPaymentLinkRequestDto, CodexPaymentPlanDto,
};
use crate::presentation::error::CommandError;
use crate::presentation::state::Repositories;
use neuradock_domain::codex::CodexAccountId;
use neuradock_infrastructure::http::openai::payment::{
    generate_payment_link, PaymentInterval, PaymentLinkRequest, PaymentPlan,
};

fn map_err(e: impl std::fmt::Display) -> CommandError {
    CommandError::from(e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn generate_codex_payment_link(
    account_id: String,
    request: CodexPaymentLinkRequestDto,
    repos: State<'_, Repositories>,
) -> Result<CodexPaymentLinkDto, CommandError> {
    let account_id = CodexAccountId::from_string(&account_id);
    let account = repos
        .codex_account
        .find_by_id(&account_id)
        .await
        .map_err(map_err)?
        .ok_or_else(|| CommandError::not_found("Codex account not found"))?;

    let payment_request = PaymentLinkRequest {
        plan: map_plan(&request.plan_type),
        country: request.country,
        currency: request.currency,
        workspace_name: request.workspace_name,
        seat_quantity: request.seat_quantity,
        price_interval: request.price_interval.map(map_interval),
    };

    let result = generate_payment_link(&account, &payment_request, None)
        .await
        .map_err(|e| CommandError::infrastructure(format!("生成支付链接失败: {}", e)))?;

    Ok(CodexPaymentLinkDto {
        url: result.url,
        session_id: result.session_id,
        plan_type: request.plan_type,
        country: result.country,
        currency: result.currency,
    })
}

fn map_plan(plan: &CodexPaymentPlanDto) -> PaymentPlan {
    match plan {
        CodexPaymentPlanDto::Plus => PaymentPlan::Plus,
        CodexPaymentPlanDto::Team => PaymentPlan::Team,
    }
}

fn map_interval(interval: CodexPaymentIntervalDto) -> PaymentInterval {
    match interval {
        CodexPaymentIntervalDto::Month => PaymentInterval::Month,
        CodexPaymentIntervalDto::Year => PaymentInterval::Year,
    }
}
