export type DemoResultKind = "app" | "window" | "file" | "utility" | "action";

export type DemoResult = {
  label: string;
  hint: string;
  kind: DemoResultKind;
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
    query: "crome",
    description: "Fuzzy typo tolerance surfaces the right app quickly.",
    results: [
      { label: "Google Chrome", hint: "App", kind: "app" },
      { label: "Chromium", hint: "App", kind: "app" },
      { label: "Chrome Settings", hint: "Window", kind: "window" }
    ]
  },
  {
    id: "windows-filter",
    query: "w term",
    description: "Prefix filters narrow results to windows only.",
    results: [
      { label: "Terminal - dev shell", hint: "Window", kind: "window" },
      { label: "Terminal - logs", hint: "Window", kind: "window" }
    ]
  },
  {
    id: "emoji",
    query: "emoji smile",
    description: "Emoji picker intent gives quick copyable symbols.",
    results: [
      { label: "😄  grinning face with smiling eyes", hint: "Emoji", kind: "utility" },
      { label: "🙂  slightly smiling face", hint: "Emoji", kind: "utility" }
    ]
  },
  {
    id: "timezone",
    query: "zurich time",
    description: "Timezone lookup handles city and alias patterns.",
    results: [
      { label: "Zurich", hint: "14:37 CET", kind: "utility" },
      { label: "Geneva", hint: "14:37 CET", kind: "utility" }
    ]
  },
  {
    id: "currency",
    query: "100usd to eur",
    description: "Currency conversion intent resolves quickly.",
    results: [{ label: "100 USD = 92.41 EUR", hint: "Currency", kind: "utility" }]
  },
  {
    id: "weather",
    query: "weather berlin",
    description: "Weather intent returns current conditions.",
    results: [{ label: "Berlin 7°C · Partly cloudy", hint: "Open-Meteo", kind: "utility" }]
  }
];
