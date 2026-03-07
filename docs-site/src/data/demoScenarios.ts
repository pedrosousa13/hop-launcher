export type DemoResultKind = "app" | "window" | "file" | "utility" | "action";
export type DemoActionKind = "open" | "focus" | "run" | "copy";

export type DemoResult = {
  label: string;
  secondaryText: string;
  kind: DemoResultKind;
  actionKind: DemoActionKind;
  leftIcon?: string;
  actionLabel?: string;
};

export type DemoScenario = {
  id: string;
  query: string;
  description: string;
  results: DemoResult[];
};

export const demoScenarios: DemoScenario[] = [
  {
    id: "app-lookup",
    query: "brav",
    description: "Client-side fuzzy behavior keeps app hits and web actions organized.",
    results: [
      {
        label: "Brave Web Browser",
        secondaryText: "Access the Internet",
        kind: "app",
        leftIcon: "/icons/apps/brave-browser.png",
        actionKind: "open"
      },
      {
        label: "Search DuckDuckGo for \"brav\"",
        secondaryText: "Web action",
        kind: "action",
        actionKind: "run"
      }
    ]
  },
  {
    id: "window-focus",
    query: "hop-launch",
    description: "Window-oriented entries expose focus actions for quick context switching.",
    results: [
      {
        label: "hop-launcher / docs-site",
        secondaryText: "Terminal - Workspace 1",
        kind: "window",
        actionKind: "focus"
      }
    ]
  },
  {
    id: "doctor-check",
    query: "doctor --strict",
    description: "Operational diagnostics surface readiness checks and failure states.",
    results: [
      {
        label: "hop-hotkeyd doctor --strict",
        secondaryText: "Control socket and shortcut state checks",
        kind: "utility",
        actionKind: "run"
      }
    ]
  },
  {
    id: "native-binding",
    query: "print-bindings",
    description: "Compositor-specific snippets are generated for native toggle wiring.",
    results: [
      {
        label: "hop-hotkeyd print-bindings",
        secondaryText: "Shows Sway/Hyprland/KDE/GNOME snippets",
        kind: "utility",
        actionKind: "copy"
      }
    ]
  },
  {
    id: "release-docs",
    query: "release runbook",
    description: "Release references stay discoverable from one docs surface.",
    results: [
      {
        label: "docs/RELEASE.md",
        secondaryText: "Maintainer release checklist",
        kind: "file",
        actionKind: "open"
      }
    ]
  }
];
