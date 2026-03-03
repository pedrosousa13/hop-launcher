export type FeatureGroup = "navigation" | "utilities" | "web";

export type FeatureCard = {
  title: string;
  summary: string;
  group: FeatureGroup;
  examples: string[];
};

export const featureCards: FeatureCard[] = [
  {
    title: "Windows + Apps",
    summary: "One query space for open windows and installed apps.",
    group: "navigation",
    examples: ["w term", "a firefox", "crome"]
  },
  {
    title: "Files",
    summary: "Indexed folders surface likely files as you type.",
    group: "navigation",
    examples: ["f report", "f invoice"]
  },
  {
    title: "Emoji",
    group: "utilities",
    summary: "Find and copy emoji quickly with natural keywords.",
    examples: ["emoji smile", ":emoji rocket"]
  },
  {
    title: "Timezone",
    summary: "City names and aliases like PST resolve local times.",
    group: "utilities",
    examples: ["zurich time", "pst"]
  },
  {
    title: "Currency",
    summary: "Convert with compact query syntax.",
    group: "utilities",
    examples: ["100usd to eur", "$100 usd to eur"]
  },
  {
    title: "Weather",
    summary: "Current weather via intent-style queries.",
    group: "utilities",
    examples: ["weather berlin", "wx 94103"]
  },
  {
    title: "Web Search Actions",
    summary: "Configurable provider actions append at result-list end.",
    group: "web",
    examples: ["Search with DuckDuckGo", "Search with Google"]
  }
];
