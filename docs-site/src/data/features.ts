export type FeatureGroup = "runtime" | "diagnostics" | "platform";

export type FeatureCard = {
  title: string;
  summary: string;
  group: FeatureGroup;
  points: string[];
};

export const featureCards: FeatureCard[] = [
  {
    title: "Fast Core Search",
    summary: "Instant results that keep up with fast typing.",
    group: "runtime",
    points: ["Apps", "Windows", "Files"]
  },
  {
    title: "Global Toggle",
    summary: "Open the launcher from anywhere with one shortcut.",
    group: "runtime",
    points: ["X11", "Wayland", "Custom shortcut"]
  },
  {
    title: "Native Linux UI",
    summary: "A clean launcher interface built for keyboard-first workflows.",
    group: "platform",
    points: ["Minimal UI", "Fast navigation"]
  },
  {
    title: "Built-In Health Checks",
    summary: "Quick diagnostics make setup and troubleshooting straightforward.",
    group: "diagnostics",
    points: ["Status", "Doctor"]
  },
  {
    title: "Works Across Desktops",
    summary: "Designed to run across popular Linux desktop environments.",
    group: "platform",
    points: ["GNOME", "KDE", "Sway", "Hyprland"]
  },
  {
    title: "Ready to Ship",
    summary: "Release flows support portable and distro-friendly packaging.",
    group: "diagnostics",
    points: ["Tarballs", "AppImage", "Deb packages"]
  }
];
