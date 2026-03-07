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
  label: string;
  query: string;
  description: string;
  results: DemoResult[];
};

export const demoScenarios: DemoScenario[] = [
  {
    id: "app-lookup",
    label: "Apps",
    query: "brav",
    description: "Find apps quickly, even with short or imperfect queries.",
    results: [
      {
        label: "Brave Web Browser",
        secondaryText: "Access the Internet",
        kind: "app",
        leftIcon: "/icons/apps/brave-browser.png",
        actionKind: "open"
      },
      {
        label: "Zen Browser",
        secondaryText: "Application",
        kind: "app",
        leftIcon: "/icons/apps/zen-browser.png",
        actionKind: "open"
      },
      {
        label: "Search DuckDuckGo for \"brav\"",
        secondaryText: "Web action",
        kind: "action",
        actionKind: "run"
      },
      {
        label: "Search Google for \"brav\"",
        secondaryText: "Web action",
        kind: "action",
        actionKind: "run"
      }
    ]
  },
  {
    id: "window-focus",
    label: "Windows",
    query: "hop-la",
    description: "Jump between open windows without breaking focus.",
    results: [
      {
        label: "hop-launcher/docs-site",
        secondaryText: "Terminal - Workspace 1",
        kind: "window",
        actionKind: "focus"
      },
      {
        label: "Search DuckDuckGo for \"hop-la\"",
        secondaryText: "Web action",
        kind: "action",
        actionKind: "run"
      }
    ]
  },
  {
    id: "file-search",
    label: "Files",
    query: "f release",
    description: "Bring files forward fast when you need them.",
    results: [
      {
        label: "docs/RELEASE.md",
        secondaryText: "Maintainer release checklist",
        kind: "file",
        actionKind: "open"
      },
      {
        label: "docs/LINUX_PACKAGING.md",
        secondaryText: "Packaging and distro publishing guide",
        kind: "file",
        actionKind: "open"
      }
    ]
  },
  {
    id: "calculator",
    label: "Calculator",
    query: "29*58",
    description: "Run quick calculations directly in the launcher.",
    results: [
      {
        label: "1682",
        secondaryText: "Calculator - 29*58",
        kind: "utility",
        actionKind: "copy"
      }
    ]
  },
  {
    id: "weather-intent",
    label: "Weather",
    query: "zurich weather",
    description: "Check weather instantly from the same search box.",
    results: [
      {
        label: "Zurich, Zurich, CH: Overcast - 11C Wind 5 km/h",
        secondaryText: "Open-Meteo - Updated 2:41 PM",
        kind: "utility",
        actionKind: "copy"
      }
    ]
  },
  {
    id: "time-intent",
    label: "Time",
    query: "tokyo time",
    description: "Get local time for any city in one step.",
    results: [
      {
        label: "Tokyo - 22:14:08",
        secondaryText: "Asia/Tokyo",
        kind: "utility",
        actionKind: "copy"
      }
    ]
  },
  {
    id: "web-search",
    label: "Web Search",
    query: "rust ipc",
    description: "Send any query straight to your preferred web search.",
    results: [
      {
        label: "Search DuckDuckGo for \"rust ipc\"",
        secondaryText: "Web action",
        kind: "action",
        actionKind: "run"
      },
      {
        label: "Search Google for \"rust ipc\"",
        secondaryText: "Web action",
        kind: "action",
        actionKind: "run"
      }
    ]
  }
];
