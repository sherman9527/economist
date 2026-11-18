import React from "react";
import {
  Activity,
  BookOpenCheck,
  BriefcaseBusiness,
  Compass,
  FlaskConical,
  FolderGit2,
  LayoutDashboard,
  PanelLeftClose,
  PanelLeftOpen,
} from "lucide-react";
import type { WorkspacePageId } from "../../types/ui";
import { SafeText } from "../common/SafeText";

export interface AppSidebarProps {
  activePage: WorkspacePageId;
  collapsed: boolean;
  unackedEconomistCount: number;
  qualifiedPicksCount: number;
  portableRootRel: string;
  blurActive: boolean;
  onSelectPage: (page: WorkspacePageId) => void;
  onToggleCollapse: () => void;
}

interface NavMeta {
  id: WorkspacePageId;
  shortZh: string;
  fullZh: string;
  icon: React.ReactNode;
  badgeType?: "picks" | "economist";
}

const NAV_ITEMS: NavMeta[] = [
  {
    id: "dashboard",
    shortZh: "组合总览",
    fullZh: "组合与策略总览",
    icon: <LayoutDashboard size={17} aria-hidden="true" />,
  },
  {
    id: "screener",
    shortZh: "轮动雷达",
    fullZh: "全A轮动与五维排雷",
    icon: <Compass size={17} aria-hidden="true" />,
    badgeType: "picks",
  },
  {
    id: "ledger",
    shortZh: "风控账本",
    fullZh: "100W模拟盘订单账本",
    icon: <BriefcaseBusiness size={17} aria-hidden="true" />,
  },
  {
    id: "factor-lab",
    shortZh: "因子插件",
    fullZh: "量化因子与参数插件",
    icon: <FlaskConical size={17} aria-hidden="true" />,
  },
  {
    id: "economist",
    shortZh: "经济学人",
    fullZh: "《经济学人》政经研报",
    icon: <BookOpenCheck size={17} aria-hidden="true" />,
    badgeType: "economist",
  },
  {
    id: "telemetry",
    shortZh: "会话巡检",
    fullZh: "Telemetry与Agent会话",
    icon: <Activity size={17} aria-hidden="true" />,
  },
];

export const AppSidebar: React.FC<AppSidebarProps> = ({
  activePage,
  collapsed,
  unackedEconomistCount,
  qualifiedPicksCount,
  portableRootRel,
  blurActive,
  onSelectPage,
  onToggleCollapse,
}) => {
  const [isMobileViewport, setIsMobileViewport] = React.useState<boolean>(() =>
    typeof window !== "undefined" && typeof window.matchMedia === "function"
      ? window.matchMedia("(max-width: 640px)").matches
      : false
  );

  React.useEffect(() => {
    if (typeof window === "undefined" || typeof window.matchMedia !== "function") return;
    const mql = window.matchMedia("(max-width: 640px)");
    const onChange = (e: MediaQueryListEvent) => setIsMobileViewport(e.matches);
    setIsMobileViewport(mql.matches);
    mql.addEventListener?.("change", onChange);
    return () => mql.removeEventListener?.("change", onChange);
  }, []);

  return (
    <>
      <aside
        aria-label="主工作区导航"
        data-blur-slot={!isMobileViewport && blurActive ? "active" : "optical"}
        className="glass-tier-1 sidebar-desktop"
        style={{
          width: collapsed ? "74px" : "238px",
          padding: "14px 10px",
          display: "flex",
          flexDirection: "column",
          justifyContent: "space-between",
          gap: "14px",
          position: "sticky",
          top: "14px",
          alignSelf: "start",
          minHeight: "calc(100vh - 28px)",
          transition: "width 200ms ease-out",
        }}
      >
        <div style={{ display: "flex", flexDirection: "column", gap: "12px", minWidth: 0 }}>
          <div
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: collapsed ? "center" : "space-between",
              gap: "8px",
              paddingInline: "6px",
              minWidth: 0,
            }}
          >
            {!collapsed && (
              <div className="min-w-zero">
                <SafeText
                  text="Liquid Quant · 全A"
                  className="lh-ui"
                  as="strong"
                />
                <div style={{ fontSize: "11px", color: "var(--text-muted)" }}>
                  小盘换手轮动 · 跨市场终端
                </div>
              </div>
            )}
            <button
              type="button"
              onClick={onToggleCollapse}
              className="glass-btn glass-btn-ghost glass-btn-sm"
              aria-label={collapsed ? "展开侧栏" : "收起侧栏"}
              title={collapsed ? "展开侧栏" : "收起侧栏"}
              style={{ paddingInline: "8px" }}
            >
              {collapsed ? <PanelLeftOpen size={15} /> : <PanelLeftClose size={15} />}
            </button>
          </div>

          <nav style={{ display: "flex", flexDirection: "column", gap: "6px", minWidth: 0 }}>
            {NAV_ITEMS.map((item) => {
              const selected = activePage === item.id;
              const badgeCount =
                item.badgeType === "economist"
                  ? unackedEconomistCount
                  : item.badgeType === "picks"
                    ? qualifiedPicksCount
                    : 0;

              return (
                <button
                  key={item.id}
                  type="button"
                  onClick={() => onSelectPage(item.id)}
                  aria-current={selected ? "page" : undefined}
                  title={item.fullZh}
                  className="glass-nav-item"
                  style={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: collapsed ? "center" : "space-between",
                    gap: "10px",
                    minHeight: "42px",
                    paddingBlock: "8px",
                    paddingInline: collapsed ? "8px" : "12px",
                    borderRadius: "14px",
                    border: selected
                      ? "1px solid color-mix(in srgb, var(--accent) 48%, transparent)"
                      : "1px solid transparent",
                    background: selected ? "var(--accent-soft)" : "transparent",
                    color: selected ? "var(--text-primary)" : "var(--text-secondary)",
                    cursor: "pointer",
                    fontSize: "13px",
                    fontWeight: selected ? 600 : 500,
                    lineHeight: 1.45,
                    minWidth: 0,
                    transition: "background 160ms ease-out, transform 160ms ease-out",
                  }}
                >
                  <span style={{ position: "relative", display: "flex", alignItems: "center", gap: "10px", minWidth: 0 }}>
                    {item.icon}
                    {!collapsed && <SafeText text={item.fullZh} />}
                    {collapsed && badgeCount > 0 && (
                      <span
                        aria-label={`${badgeCount}`}
                        className="tabular-nums"
                        style={{
                          position: "absolute",
                          top: "-5px",
                          right: "-7px",
                          minWidth: "16px",
                          height: "16px",
                          paddingInline: "4px",
                          borderRadius: "999px",
                          fontSize: "10px",
                          lineHeight: "16px",
                          textAlign: "center",
                          fontWeight: 700,
                          background: item.badgeType === "economist" ? "var(--risk-stoploss-bg)" : "var(--risk-clear-bg)",
                          color: item.badgeType === "economist" ? "var(--risk-stoploss-text)" : "var(--risk-clear-text)",
                          border: "1px solid var(--glass-rim-bright)",
                        }}
                      >
                        {badgeCount}
                      </span>
                    )}
                  </span>
                  {!collapsed && badgeCount > 0 && (
                    <span
                      className={`glass-pill tabular-nums ${
                        item.badgeType === "economist" ? "pill-risk-stoploss" : "pill-risk-clear"
                      }`}
                      style={{ paddingInline: "7px", minHeight: "22px", fontSize: "11px" }}
                    >
                      {badgeCount}
                    </span>
                  )}
                </button>
              );
            })}
          </nav>
        </div>

        <div
          className="glass-optical-card"
          style={{ padding: collapsed ? "8px 6px" : "10px 12px", textAlign: collapsed ? "center" : "left" }}
        >
          <div style={{ display: "flex", alignItems: "center", justifyContent: collapsed ? "center" : "flex-start", gap: "6px" }}>
            <FolderGit2 size={14} style={{ color: "var(--accent)", flexShrink: 0 }} />
            {!collapsed && (
              <span className="tabular-nums" style={{ fontSize: "11.5px", color: "var(--text-secondary)" }}>
                便携根路径 `{portableRootRel}`
              </span>
            )}
          </div>
          {!collapsed && (
            <div style={{ fontSize: "11px", color: "var(--text-muted)", marginTop: "4px" }}>
              零C盘写入 · SQLite WAL 就绪
            </div>
          )}
        </div>
      </aside>

      {/* Mobile <640px Responsive Bottom Glass Bar */}
      <nav
        aria-label="移动端底部导航"
        data-blur-slot={isMobileViewport && blurActive ? "active" : "optical"}
        className="glass-tier-1 sidebar-mobile-tabs"
        style={{
          position: "fixed",
          bottom: "8px",
          left: "8px",
          right: "8px",
          zIndex: 45,
          padding: "6px",
          gap: "4px",
          justifyContent: "space-around",
        }}
      >
        {NAV_ITEMS.map((item) => {
          const selected = activePage === item.id;
          const mobileBadge =
            item.badgeType === "economist"
              ? unackedEconomistCount
              : item.badgeType === "picks"
                ? qualifiedPicksCount
                : 0;
          return (
            <button
              key={item.id}
              type="button"
              onClick={() => onSelectPage(item.id)}
              aria-current={selected ? "page" : undefined}
              style={{
                position: "relative",
                flex: "1 1 0",
                minWidth: 0,
                minHeight: "44px",
                paddingBlock: "5px",
                paddingInline: "4px",
                borderRadius: "12px",
                border: "none",
                background: selected ? "var(--accent-soft)" : "transparent",
                color: selected ? "var(--text-primary)" : "var(--text-secondary)",
                display: "flex",
                flexDirection: "column",
                alignItems: "center",
                gap: "2px",
                fontSize: "11px",
                lineHeight: 1.35,
                cursor: "pointer",
              }}
            >
              {item.icon}
              {mobileBadge > 0 && (
                <span
                  className="tabular-nums"
                  style={{
                    position: "absolute",
                    top: "2px",
                    right: "6px",
                    minWidth: "15px",
                    height: "15px",
                    paddingInline: "3px",
                    borderRadius: "999px",
                    fontSize: "9.5px",
                    lineHeight: "15px",
                    fontWeight: 700,
                    background: item.badgeType === "economist" ? "var(--risk-stoploss-bg)" : "var(--risk-clear-bg)",
                    color: item.badgeType === "economist" ? "var(--risk-stoploss-text)" : "var(--risk-clear-text)",
                  }}
                >
                  {mobileBadge}
                </span>
              )}
              <span className="safe-text-single">{item.shortZh}</span>
            </button>
          );
        })}
      </nav>
    </>
  );
};
