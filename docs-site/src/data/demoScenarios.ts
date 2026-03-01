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
    id: "typo-app",
    query: "brav",
    description: "Apps stay readable with left app icons and right-side action affordances.",
    results: [
      {
        label: "Brave Web Browser",
        secondaryText: "Access the Internet",
        kind: "app",
        leftIcon: "/icons/apps/brave-browser.png",
        actionKind: "open"
      },
      {
        label: "Search Google for \"brav\"",
        secondaryText: "Web action",
        kind: "action",
        actionKind: "run"
      },
      {
        label: "Search DuckDuckGo for \"brav\"",
        secondaryText: "Web action",
        kind: "action",
        actionKind: "run"
      },
      {
        label: "Zen Browser",
        secondaryText: "Application",
        kind: "app",
        leftIcon: "/icons/apps/zen-browser.png",
        actionKind: "open"
      },
      {
        label: "Chromium Web Browser",
        secondaryText: "Access the Internet",
        kind: "app",
        leftIcon: "/icons/apps/chromium.png",
        actionKind: "open"
      }
    ]
  },
  {
    id: "windows-filter",
    query: "hop-la",
    description: "Window-focused queries show monitor icon + Focus action on the right.",
    results: [
      {
        label: "..hop-launcher/docs-site",
        secondaryText: "Warp - Workspace 1",
        kind: "window",
        actionKind: "focus"
      },
      {
        label: "Search Google for \"hop-la\"",
        secondaryText: "Web action",
        kind: "action",
        actionKind: "run"
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
    id: "calculator",
    query: "29*58",
    description: "Utility rows keep copy affordance explicit and aligned right.",
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
    id: "timezone",
    query: "zurich time",
    description: "Timezone results keep primary value and context in a two-line stack.",
    results: [
      {
        label: "Zurich - 14:40:47",
        secondaryText: "Europe/Zurich",
        kind: "utility",
        actionKind: "copy"
      }
    ]
  },
  {
    id: "weather",
    query: "zurich weather",
    description: "Weather intent mirrors launcher row hierarchy and copy affordance.",
    results: [
      {
        label: "Zurich, Zurich, CH: Overcast - 11C Wind 5 km/h",
        secondaryText: "Open-Meteo - Updated 2:41:02 PM",
        kind: "utility",
        actionKind: "copy"
      }
    ]
  }
];
