use devsweep_core::software::{
    SoftwareEligibilityReason, SoftwareExecutionOutcome, SoftwareIdentity, SoftwareSizeBasis,
    SoftwareSizeEvidence,
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::{
    i18n::Locale,
    tui::{
        app::App,
        modes::software::{SoftwarePhase, selectable},
    },
};

use super::{format::format_bytes, theme::*};

pub(super) fn render_software(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(8),
            Constraint::Length(4),
            Constraint::Length(1),
        ])
        .split(area);
    render_header(frame, chunks[0], app);
    render_body(frame, chunks[1], app);
    render_summary(frame, chunks[2], app);
    render_footer(frame, chunks[3], app);
    if app.software.phase == SoftwarePhase::Confirming {
        render_confirmation(frame, app);
    }
}

fn render_header(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let zh = app.shell.locale == Locale::ZhCn;
    let phase = phase_label(app.software.phase, zh);
    let count = app
        .software
        .inventory
        .as_ref()
        .map_or(0, |value| value.entries.len());
    let query = if app.filter_active {
        format!(
            "  {}: {}_",
            if zh { "搜索" } else { "Search" },
            app.software.query
        )
    } else if app.software.query.is_empty() {
        String::new()
    } else {
        format!(
            "  {}: {}",
            if zh { "搜索" } else { "Search" },
            app.software.query
        )
    };
    let lines = vec![
        Line::from(vec![
            Span::styled(app.shell.app_title.clone(), accent_style()),
            Span::styled(
                if zh {
                    "  软件工作台  |  "
                } else {
                    "  Software workbench  |  "
                },
                panel_style(),
            ),
            Span::styled(phase, warning_style()),
            Span::styled(query, muted_style()),
        ]),
        Line::from(if zh {
            format!("{count} 项 · 仅当前用户 MSIX 可选 · MSI/ARP 始终为手动 · 最近使用时间：未知")
        } else {
            format!(
                "{count} entries · only current-user MSIX is selectable · MSI/ARP stays manual · last used: unknown"
            )
        }),
    ];
    frame.render_widget(
        Paragraph::new(lines).block(focused_panel_block(if zh { "软件" } else { "Software" })),
        area,
    );
}

fn render_body(frame: &mut Frame<'_>, area: Rect, app: &App) {
    if area.width < 86 {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
            .split(area);
        render_list(frame, chunks[0], app);
        render_details(frame, chunks[1], app);
    } else {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
            .split(area);
        render_list(frame, chunks[0], app);
        render_details(frame, chunks[1], app);
    }
}

fn render_list(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let zh = app.shell.locale == Locale::ZhCn;
    let entries = app.software.visible_entries();
    let items = entries
        .iter()
        .map(|entry| {
            let selected = app.software.selected_ids.contains(&entry.id);
            let mark = if selectable(entry) {
                if selected { "[x]" } else { "[ ]" }
            } else {
                "[-]"
            };
            let name = entry.display_name.as_deref().unwrap_or(&entry.id);
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{mark} "),
                    if selected {
                        accent_style()
                    } else {
                        muted_style()
                    },
                ),
                Span::styled(name.to_string(), panel_style()),
                Span::raw("  "),
                Span::styled(size_label(&entry.size, zh), warning_style()),
                Span::raw("  "),
                Span::styled(reason_label(entry.eligibility.reason, zh), muted_style()),
            ]))
        })
        .collect::<Vec<_>>();
    let mut state =
        ListState::default().with_selected((!entries.is_empty()).then_some(app.software.cursor));
    let list = List::new(items)
        .block(panel_block(if zh {
            "可访问软件列表"
        } else {
            "Accessible software list"
        }))
        .highlight_style(selected_row_style().add_modifier(Modifier::BOLD))
        .highlight_symbol("› ");
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_details(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let zh = app.shell.locale == Locale::ZhCn;
    let mut lines = if let Some(entry) = app.software.focused_entry() {
        vec![
            Line::from(Span::styled(
                entry
                    .display_name
                    .clone()
                    .unwrap_or_else(|| entry.id.clone()),
                accent_style(),
            )),
            Line::from(format!("ID: {}", entry.id)),
            Line::from(format!(
                "{}: {}",
                if zh { "精确标识" } else { "Exact identity" },
                identity(&entry.identity)
            )),
            Line::from(format!(
                "{}: {}",
                if zh { "范围" } else { "Scope" },
                if matches!(
                    entry.scope,
                    devsweep_core::software::SoftwareScope::CurrentUser
                ) {
                    if zh { "当前用户" } else { "current user" }
                } else if zh {
                    "计算机"
                } else {
                    "machine"
                }
            )),
            Line::from(format!(
                "{}: {}",
                if zh { "大小" } else { "Size" },
                size_label(&entry.size, zh)
            )),
            Line::from(format!(
                "{}: {}",
                if zh { "最近使用" } else { "Last used" },
                if zh {
                    "未知（无受支持的精确来源）"
                } else {
                    "unknown (no supported exact source)"
                }
            )),
            Line::from(format!(
                "{}: {}",
                if zh { "资格" } else { "Eligibility" },
                reason_label(entry.eligibility.reason, zh)
            )),
        ]
    } else {
        vec![Line::from(if zh {
            "刷新清单并选择一项以查看精确标识。"
        } else {
            "Refresh inventory and focus an entry to inspect its exact identity."
        })]
    };
    if let Some(report) = &app.software.report {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            if zh {
                "终态结果"
            } else {
                "Terminal results"
            },
            accent_style(),
        )));
        for outcome in &report.outcomes {
            lines.push(Line::from(format!(
                "{} — {}",
                outcome.software_id,
                outcome_label(outcome.outcome, zh)
            )));
        }
    }
    if let Some(error) = &app.software.error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(error.clone(), error_style())));
    }
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(panel_block(if zh { "详情" } else { "Details" }))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_summary(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let zh = app.shell.locale == Locale::ZhCn;
    let mut estimated = 0_u64;
    let mut measured = 0_u64;
    let mut lower = 0_u64;
    let mut unknown = 0_usize;
    if let Some(inventory) = &app.software.inventory {
        for entry in &inventory.entries {
            match entry.size {
                SoftwareSizeEvidence::Available {
                    value_bytes,
                    basis: SoftwareSizeBasis::ReportedEstimate,
                    ..
                } => estimated = estimated.saturating_add(value_bytes),
                SoftwareSizeEvidence::Available {
                    value_bytes,
                    basis:
                        SoftwareSizeBasis::MeasuredInstalledLocation
                        | SoftwareSizeBasis::MeasuredDirectory,
                    ..
                } => measured = measured.saturating_add(value_bytes),
                SoftwareSizeEvidence::Partial {
                    lower_bound_bytes, ..
                } => lower = lower.saturating_add(lower_bound_bytes),
                SoftwareSizeEvidence::Unknown { .. } => unknown += 1,
            }
        }
    }
    let digest = app
        .software
        .preview
        .as_ref()
        .map_or("—", |preview| preview.digest.as_str());
    let text = if zh {
        format!(
            "已选 {} · 报告估计 {} · 实测 {} · 下限 ≥ {} · 未知 {}\n预览摘要 {} · 不可逆：DevSweep 无法还原或重新安装软件",
            app.software.selected_ids.len(),
            format_bytes(estimated),
            format_bytes(measured),
            format_bytes(lower),
            unknown,
            digest
        )
    } else {
        format!(
            "Selected {} · reported estimate {} · measured {} · lower bound ≥ {} · unknown {}\nPreview digest {} · irreversible: DevSweep cannot restore or reinstall software",
            app.software.selected_ids.len(),
            format_bytes(estimated),
            format_bytes(measured),
            format_bytes(lower),
            unknown,
            digest
        )
    };
    frame.render_widget(
        Paragraph::new(text).block(focused_panel_block(if zh {
            "固定摘要"
        } else {
            "Sticky summary"
        })),
        area,
    );
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let zh = app.shell.locale == Locale::ZhCn;
    let actions = if app.software.phase == SoftwarePhase::Confirming {
        if zh {
            " 确认  [Enter] 执行不可逆卸载  [Esc] 返回"
        } else {
            " CONFIRM  [Enter] run irreversible uninstall  [Esc] back"
        }
    } else if app.software.operation_id.is_some() {
        if zh {
            " 运行中  [x] 请求取消并等待"
        } else {
            " RUNNING  [x] request cancel and wait"
        }
    } else if zh {
        " 软件  [i] 清单  [Space] 选择  [a] 全选  [v] 预览  [r] 恢复审计  [Enter] 确认  [/] 搜索  [p] 语言  [q] 退出"
    } else {
        " SOFTWARE  [i] inventory  [Space] select  [a] all  [v] preview  [r] recover audit  [Enter] confirm  [/] search  [p] language  [q] quit"
    };
    frame.render_widget(
        Paragraph::new(actions).style(Style::default().bg(FOOTER_SURFACE).fg(TEXT_STRONG)),
        area,
    );
}

fn render_confirmation(frame: &mut Frame<'_>, app: &App) {
    let zh = app.shell.locale == Locale::ZhCn;
    let area = centered_rect(76, 72, frame.area());
    let mut lines = vec![
        Line::from(Span::styled(
            if zh {
                "不可逆软件卸载"
            } else {
                "Irreversible software uninstall"
            },
            error_style(),
        )),
        Line::from(if zh {
            "DevSweep 无法还原或重新安装软件。请核对每个精确标识。"
        } else {
            "DevSweep cannot restore or reinstall software. Review every exact identity."
        }),
        Line::from(""),
    ];
    if let Some(preview) = &app.software.preview {
        for item in &preview.selected {
            lines.push(Line::from(format!("• {}", identity(&item.identity))));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(format!("digest: {}", preview.digest)));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(if zh {
        "Enter：确认并执行    Esc：返回预览"
    } else {
        "Enter: confirm and execute    Esc: return to preview"
    }));
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(if zh {
                "第二次确认"
            } else {
                "Second confirmation"
            }))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

fn phase_label(phase: SoftwarePhase, zh: bool) -> &'static str {
    match (phase, zh) {
        (SoftwarePhase::Loading, false) => "loading",
        (SoftwarePhase::Loading, true) => "加载中",
        (SoftwarePhase::Ready, false) => "ready",
        (SoftwarePhase::Ready, true) => "就绪",
        (SoftwarePhase::Selecting, false) => "selecting",
        (SoftwarePhase::Selecting, true) => "选择中",
        (SoftwarePhase::Previewing, false) => "previewing",
        (SoftwarePhase::Previewing, true) => "预览中",
        (SoftwarePhase::PreviewReady, false) => "preview ready",
        (SoftwarePhase::PreviewReady, true) => "预览就绪",
        (SoftwarePhase::Confirming, false) => "confirming",
        (SoftwarePhase::Confirming, true) => "确认中",
        (SoftwarePhase::Uninstalling, false) => "uninstalling",
        (SoftwarePhase::Uninstalling, true) => "卸载中",
        (SoftwarePhase::Terminal, false) => "terminal",
        (SoftwarePhase::Terminal, true) => "已终止",
        (SoftwarePhase::Unknown, false) => "unknown",
        (SoftwarePhase::Unknown, true) => "未知",
        (SoftwarePhase::Canceling, false) => "canceling",
        (SoftwarePhase::Canceling, true) => "取消中",
    }
}

fn identity(identity: &SoftwareIdentity) -> String {
    match identity {
        SoftwareIdentity::Arp { hive, view, subkey } => format!("ARP:{hive:?}:{view:?}:{subkey}"),
        SoftwareIdentity::Msi {
            product_code,
            context,
        } => format!("MSI:{context:?}:{product_code}"),
        SoftwareIdentity::Msix { package_full_name } => format!("MSIX:{package_full_name}"),
    }
}

fn size_label(size: &SoftwareSizeEvidence, zh: bool) -> String {
    match size {
        SoftwareSizeEvidence::Available {
            value_bytes,
            basis: SoftwareSizeBasis::ReportedEstimate,
            ..
        } => format!(
            "{} ({})",
            format_bytes(*value_bytes),
            if zh {
                "报告估计"
            } else {
                "reported estimate"
            }
        ),
        SoftwareSizeEvidence::Available {
            value_bytes,
            basis:
                SoftwareSizeBasis::MeasuredInstalledLocation | SoftwareSizeBasis::MeasuredDirectory,
            ..
        } => format!(
            "{} ({})",
            format_bytes(*value_bytes),
            if zh { "实测" } else { "measured" }
        ),
        SoftwareSizeEvidence::Partial {
            lower_bound_bytes, ..
        } => format!(
            "≥ {} ({})",
            format_bytes(*lower_bound_bytes),
            if zh { "下限" } else { "lower bound" }
        ),
        SoftwareSizeEvidence::Unknown { .. } => if zh { "未知" } else { "unknown" }.to_string(),
    }
}

fn reason_label(reason: SoftwareEligibilityReason, zh: bool) -> &'static str {
    use SoftwareEligibilityReason::*;
    match (reason, zh) {
        (ProtectedProduct, false) => "protected product",
        (ProtectedProduct, true) => "受保护产品",
        (SourceIncomplete, false) => "source incomplete",
        (SourceIncomplete, true) => "来源不完整",
        (ConflictingIdentity, false) => "conflicting identity",
        (ConflictingIdentity, true) => "标识冲突",
        (NoRemove, false) => "removal disabled",
        (NoRemove, true) => "禁止卸载",
        (HiddenEntry, false) => "hidden entry",
        (HiddenEntry, true) => "隐藏项",
        (SystemOrUpdate, false) => "system/update",
        (SystemOrUpdate, true) => "系统或更新",
        (DependencyPackage, false) => "dependency package",
        (DependencyPackage, true) => "依赖包",
        (StubPackage, false) => "stub package",
        (StubPackage, true) => "存根包",
        (UnhealthyPackage, false) => "unhealthy package",
        (UnhealthyPackage, true) => "异常包",
        (MsiExecutionNotSupportedV1, false) => "MSI uninstall is manual in Software V1",
        (MsiExecutionNotSupportedV1, true) => "Software V1 仅手动处理 MSI 卸载",
        (RegistryOnlyManual, false) => "registry-only entry is manual",
        (RegistryOnlyManual, true) => "仅注册表项需手动处理",
        (UnsupportedSource, false) => "unsupported source",
        (UnsupportedSource, true) => "不支持的来源",
        (EligibleCurrentUserMsix, false) => "eligible current-user MSIX",
        (EligibleCurrentUserMsix, true) => "符合条件的当前用户 MSIX",
    }
}

fn outcome_label(outcome: SoftwareExecutionOutcome, zh: bool) -> &'static str {
    match (outcome, zh) {
        (SoftwareExecutionOutcome::Removed, false) => "removed",
        (SoftwareExecutionOutcome::Removed, true) => "已移除",
        (SoftwareExecutionOutcome::RebootRequired, false) => "reboot required",
        (SoftwareExecutionOutcome::RebootRequired, true) => "需要重启",
        (SoftwareExecutionOutcome::StillPresent, false) => "still present",
        (SoftwareExecutionOutcome::StillPresent, true) => "仍存在",
        (SoftwareExecutionOutcome::Failed, false) => "failed",
        (SoftwareExecutionOutcome::Failed, true) => "失败",
        (SoftwareExecutionOutcome::UnknownAfterDispatch, false) => "unknown after dispatch",
        (SoftwareExecutionOutcome::UnknownAfterDispatch, true) => "分派后未知",
        (SoftwareExecutionOutcome::CanceledBeforeStart, false) => "canceled before start",
        (SoftwareExecutionOutcome::CanceledBeforeStart, true) => "启动前取消",
    }
}
