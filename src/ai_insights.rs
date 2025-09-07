use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetInsights {
    pub asset: String,
    pub trading_notes: Vec<String>,
    pub risk_assessment: String,
    pub execution_recommendations: Vec<String>,
    pub market_context: String,
}

#[derive(Debug, Clone, Deserialize)]
struct AssetInsightsResponse {
    pub trading_notes: Vec<String>,
    pub risk_assessment: String,
    pub execution_recommendations: Vec<String>,
    pub market_context: String,
}

#[derive(Debug, Clone)]
pub struct AssetMetrics {
    pub asset: String,
    pub total_return: f64,
    pub sharpe_ratio: f64,
    pub win_rate: f64,
    pub max_drawdown: f64,
    pub trading_days: u32,
    pub profit_factor: f64,
    pub current_price: f64,
    pub ma30: f64,
    pub ma7: f64,
    pub rs_ma7: f64,
    pub rs_ma30: f64,
    pub atr_14: f64,
    pub volatility: f64,
}

/// Generate AI-powered insights for a trading asset based on its performance data
pub async fn generate_asset_insights(metrics: &AssetMetrics) -> Result<AssetInsights> {
    // Check if OpenAI API key is available
    let api_key = match env::var("OPENAI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            println!(
                "⚠️  OPENAI_API_KEY not set, using fallback analysis for {}",
                metrics.asset
            );
            return Ok(generate_fallback_insights(
                &metrics.asset,
                metrics.total_return,
                metrics.sharpe_ratio,
                metrics.win_rate,
                metrics.max_drawdown,
            ));
        }
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let prompt = format!(
        r#"You are a quantitative trading analyst specializing in cryptocurrency momentum strategies. Analyze this trading strategy performance and provide actionable insights.

ASSET: {}
PERFORMANCE METRICS:
- Total Return: {:.2}%
- Sharpe Ratio: {:.2}
- Win Rate: {:.1}%
- Max Drawdown: {:.2}%
- Trading Days: {}
- Profit Factor: {:.2}

CURRENT MARKET DATA:
- Current Price: ${:.2}
- MA30: ${:.2}
- MA7: ${:.2}
- RS vs BTC (MA7): {:.2}
- RS vs BTC (MA30): {:.2}
- ATR(14): ${:.2}
- Volatility: {:.2}%

Please provide:
1. 3-5 specific trading notes (execution tips, market conditions, risk factors)
2. Risk assessment (1-2 sentences on risk level and key concerns)
3. 2-3 execution recommendations (entry/exit strategies, position sizing)
4. Market context (1-2 sentences on current market conditions and outlook)

IMPORTANT: Respond with ONLY valid JSON in this exact format (no markdown, no explanations, no code blocks):
{{
  "trading_notes": ["note1", "note2", "note3"],
  "risk_assessment": "brief risk summary",
  "execution_recommendations": ["rec1", "rec2"],
  "market_context": "market outlook"
}}"#,
        metrics.asset,
        metrics.total_return,
        metrics.sharpe_ratio,
        metrics.win_rate,
        metrics.max_drawdown,
        metrics.trading_days,
        metrics.profit_factor,
        metrics.current_price,
        metrics.ma30,
        metrics.ma7,
        metrics.rs_ma7,
        metrics.rs_ma30,
        metrics.atr_14,
        metrics.volatility
    );

    let request_body = serde_json::json!({
        "model": "gpt-4o-mini",
        "messages": [
            {
                "role": "user",
                "content": prompt
            }
        ],
        "temperature": 0.7,
        "max_tokens": 1000
    });

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        println!(
            "⚠️  OpenAI API error for {}: {}. Using fallback analysis.",
            metrics.asset, error_text
        );
        return Ok(generate_fallback_insights(
            &metrics.asset,
            metrics.total_return,
            metrics.sharpe_ratio,
            metrics.win_rate,
            metrics.max_drawdown,
        ));
    }

    let response_json: serde_json::Value = response.json().await?;

    // Check if the response has the expected structure
    let choices = response_json["choices"].as_array()
        .ok_or_else(|| anyhow::anyhow!("Invalid response structure: no choices array"))?;

    if choices.is_empty() {
        return Err(anyhow::anyhow!("No choices in OpenAI response"));
    }

    let content = choices[0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No content in OpenAI response"))?;

    // Debug: Print the raw content to understand what we're getting
    if content.trim().is_empty() {
        println!(
            "⚠️  Empty OpenAI response for {}. Using fallback analysis.",
            metrics.asset
        );
        return Ok(generate_fallback_insights(
            &metrics.asset,
            metrics.total_return,
            metrics.sharpe_ratio,
            metrics.win_rate,
            metrics.max_drawdown,
        ));
    }

    // Try to extract JSON from the response if it's wrapped in markdown code blocks
    let json_content = if content.trim().starts_with("```json") {
        // Extract content between ```json and ```
        let start = content.find("```json").unwrap_or(0) + 7;
        let end = content.rfind("```").unwrap_or(content.len());
        content[start..end].trim()
    } else if content.trim().starts_with("```") {
        // Extract content between ``` and ```
        let start = content.find("```").unwrap_or(0) + 3;
        let end = content.rfind("```").unwrap_or(content.len());
        content[start..end].trim()
    } else {
        content.trim()
    };

    // Parse the JSON response
    match serde_json::from_str::<AssetInsightsResponse>(json_content) {
        Ok(response) => Ok(AssetInsights {
            asset: metrics.asset.clone(),
            trading_notes: response.trading_notes,
            risk_assessment: response.risk_assessment,
            execution_recommendations: response.execution_recommendations,
            market_context: response.market_context,
        }),
        Err(e) => {
            println!(
                "⚠️  Failed to parse OpenAI response for {}: {}. Raw content: '{}'. Using fallback analysis.",
                metrics.asset, e, json_content
            );
            Ok(generate_fallback_insights(
                &metrics.asset,
                metrics.total_return,
                metrics.sharpe_ratio,
                metrics.win_rate,
                metrics.max_drawdown,
            ))
        }
    }
}

/// Generate portfolio-level insights based on overall strategy performance
pub async fn generate_portfolio_insights(
    total_strategies: usize,
    profitable_strategies: usize,
    avg_return: f64,
    avg_sharpe: f64,
    avg_win_rate: f64,
    top_performers: Vec<(String, f64)>,
    market_conditions: &str,
) -> Result<String> {
    // Check if OpenAI API key is available
    let api_key = match env::var("OPENAI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            let success_rate = (profitable_strategies as f64 / total_strategies as f64) * 100.0;
            return Ok(format!(
                "Portfolio Analysis: {} profitable strategies out of {} total ({:.1}% success rate). \
                 Average return: {:.1}%, Average Sharpe: {:.2}, Average win rate: {:.1}%. \
                 Market conditions: {}. Top performers show strong momentum characteristics.",
                profitable_strategies,
                total_strategies,
                success_rate,
                avg_return,
                avg_sharpe,
                avg_win_rate,
                market_conditions
            ));
        }
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let top_performers_str = top_performers
        .iter()
        .take(5)
        .map(|(asset, return_pct)| format!("{}: {:.1}%", asset, return_pct))
        .collect::<Vec<_>>()
        .join(", ");

    let prompt = format!(
        r#"You are a quantitative portfolio manager specializing in cryptocurrency momentum strategies. Analyze this portfolio performance and provide market insights.

PORTFOLIO METRICS:
- Total Strategies: {}
- Profitable Strategies: {} ({:.1}%)
- Average Return (Profitable): {:.1}%
- Average Sharpe Ratio: {:.2}
- Average Win Rate: {:.1}%

TOP PERFORMERS: {}

MARKET CONDITIONS: {}

Provide a 2-3 paragraph analysis covering:
1. Overall strategy effectiveness and market conditions
2. Key themes in the top performers
3. Risk management recommendations
4. Market outlook and positioning advice

Be specific and actionable for a quantitative trader."#,
        total_strategies,
        profitable_strategies,
        (profitable_strategies as f64 / total_strategies as f64) * 100.0,
        avg_return,
        avg_sharpe,
        avg_win_rate,
        top_performers_str,
        market_conditions
    );

    let request_body = serde_json::json!({
        "model": "gpt-4o-mini",
        "messages": [
            {
                "role": "user",
                "content": prompt
            }
        ],
        "temperature": 0.8,
        "max_tokens": 800
    });

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        println!(
            "⚠️  OpenAI API error for portfolio analysis: {}. Using fallback analysis.",
            error_text
        );
        let success_rate = (profitable_strategies as f64 / total_strategies as f64) * 100.0;
        return Ok(format!(
            "Portfolio Analysis: {} profitable strategies out of {} total ({:.1}% success rate). \
             Average return: {:.1}%, Average Sharpe: {:.2}, Average win rate: {:.1}%. \
             Market conditions: {}. Top performers show strong momentum characteristics.",
            profitable_strategies,
            total_strategies,
            success_rate,
            avg_return,
            avg_sharpe,
            avg_win_rate,
            market_conditions
        ));
    }

    let response_json: serde_json::Value = response.json().await?;
    let content = response_json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No content in OpenAI response"))?;

    Ok(content.to_string())
}

/// Generate market context based on current market data
pub async fn generate_market_context(
    btc_price: f64,
    eth_price: f64,
    market_cap_change: f64,
    fear_greed_index: Option<i32>,
) -> Result<String> {
    // Check if OpenAI API key is available
    let api_key = match env::var("OPENAI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            let sentiment = if market_cap_change > 5.0 {
                "bullish"
            } else if market_cap_change < -5.0 {
                "bearish"
            } else {
                "neutral"
            };
            return Ok(format!(
                "Market Context: BTC at ${:.2}, ETH at ${:.2}, 24h change: {:.2}%. Market sentiment appears {}. \
                 Fear & Greed Index: {}. Momentum strategies may benefit from current market structure.",
                btc_price,
                eth_price,
                market_cap_change,
                sentiment,
                fear_greed_index
                    .map(|i| i.to_string())
                    .unwrap_or_else(|| "N/A".to_string())
            ));
        }
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let fear_greed_str = match fear_greed_index {
        Some(index) => format!("Fear & Greed Index: {}", index),
        None => "Fear & Greed Index: Not available".to_string(),
    };

    let prompt = format!(
        r#"You are a crypto market analyst. Provide a brief market context based on current data.

CURRENT MARKET DATA:
- BTC Price: ${:.2}
- ETH Price: ${:.2}
- Market Cap Change (24h): {:.2}%
- {}

Provide a 1-2 sentence market context focusing on:
- Overall market sentiment
- Key support/resistance levels
- Risk factors for momentum strategies
- Market structure insights

Be concise and actionable for traders."#,
        btc_price, eth_price, market_cap_change, fear_greed_str
    );

    let request_body = serde_json::json!({
        "model": "gpt-4o-mini",
        "messages": [
            {
                "role": "user",
                "content": prompt
            }
        ],
        "temperature": 0.6,
        "max_tokens": 300
    });

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        println!(
            "⚠️  OpenAI API error for market context: {}. Using fallback analysis.",
            error_text
        );
        let sentiment = if market_cap_change > 5.0 {
            "bullish"
        } else if market_cap_change < -5.0 {
            "bearish"
        } else {
            "neutral"
        };
        return Ok(format!(
            "Market Context: BTC at ${:.2}, ETH at ${:.2}, 24h change: {:.2}%. Market sentiment appears {}. \
             Fear & Greed Index: {}. Momentum strategies may benefit from current market structure.",
            btc_price,
            eth_price,
            market_cap_change,
            sentiment,
            fear_greed_index
                .map(|i| i.to_string())
                .unwrap_or_else(|| "N/A".to_string())
        ));
    }

    let response_json: serde_json::Value = response.json().await?;
    let content = response_json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No content in OpenAI response"))?;

    Ok(content.to_string())
}

/// Fallback function when OpenAI API is not available
pub fn generate_fallback_insights(
    asset: &str,
    total_return: f64,
    sharpe_ratio: f64,
    win_rate: f64,
    max_drawdown: f64,
) -> AssetInsights {
    let mut trading_notes = Vec::new();
    let risk_assessment;
    let mut execution_recommendations = Vec::new();

    // Generate basic insights based on performance metrics
    if total_return > 1000.0 {
        trading_notes.push(
            "Exceptional momentum - consider scaling in gradually to manage volatility risk"
                .to_string(),
        );
        risk_assessment = "High return potential but extreme volatility risk".to_string();
    } else if total_return > 100.0 {
        trading_notes.push("Strong momentum trend - monitor for continuation signals".to_string());
        risk_assessment = "High return with moderate volatility".to_string();
    } else if total_return > 10.0 {
        trading_notes
            .push("Solid performance - suitable for core portfolio allocation".to_string());
        risk_assessment = "Moderate risk with good return potential".to_string();
    } else {
        trading_notes.push("Conservative performance - consider for risk management".to_string());
        risk_assessment = "Low risk, modest returns".to_string();
    }

    if sharpe_ratio > 2.0 {
        trading_notes.push("Excellent risk-adjusted returns - increase position size".to_string());
        execution_recommendations
            .push("Consider larger position size due to high Sharpe ratio".to_string());
    } else if sharpe_ratio > 1.0 {
        trading_notes.push("Good risk-adjusted performance - maintain current sizing".to_string());
        execution_recommendations.push("Standard position sizing appropriate".to_string());
    } else {
        trading_notes
            .push("Lower risk-adjusted returns - consider reducing position size".to_string());
        execution_recommendations
            .push("Consider smaller position size due to lower Sharpe ratio".to_string());
    }

    if win_rate > 80.0 {
        trading_notes.push("High win rate suggests strong signal quality".to_string());
    } else if win_rate < 50.0 {
        trading_notes
            .push("Low win rate - review entry criteria and market conditions".to_string());
    }

    if max_drawdown > 20.0 {
        trading_notes.push("High drawdown risk - implement strict stop losses".to_string());
        execution_recommendations
            .push("Use tighter stop losses to manage drawdown risk".to_string());
    }

    AssetInsights {
        asset: asset.to_string(),
        trading_notes,
        risk_assessment,
        execution_recommendations,
        market_context: "Market analysis unavailable - using fallback metrics".to_string(),
    }
}
