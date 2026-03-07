export type FeatureGroup = "runtime" | "diagnostics" | "platform";

export type FeatureCard = {
  title: string;
  summary: string;
  group: FeatureGroup;
  examples: string[];
};

export const featureCards: FeatureCard[] = [
  {
    title: "hopd IPC Runtime",
    summary: "A Rust daemon serving launcher control and query IPC over a local socket.",
    group: "runtime",
    examples: ["cargo test -p hopd", "hopd user service", "socket health checks"]
  },
  {
    title: "hop-hotkeyd Shortcut Control",
    summary: "Compositor-aware global toggle handling with runtime shortcut configuration.",
    group: "runtime",
    examples: ["hop-hotkeyd config set", "hop-hotkeyd setup-shortcut", "hop-hotkeyd repair-shortcut"]
  },
  {
    title: "GTK Launcher Frontend",
    summary: "Native GTK client talking to daemon services for responsive launcher interactions.",
    group: "platform",
    examples: ["cargo run --features gtk_ui", "scripts/run-gtk-manual-test.sh"]
  },
  {
    title: "Doctor + Status Diagnostics",
    summary: "Structured health output for sockets, native backend readiness, and shortcut drift.",
    group: "diagnostics",
    examples: ["hop-hotkeyd status", "hop-hotkeyd doctor --strict"]
  },
  {
    title: "Compositor Bridges",
    summary: "Native toggle bridge support for Sway, Hyprland, KDE Wayland, and GNOME Wayland.",
    group: "platform",
    examples: ["sway send_tick", "hyprctl dispatch event", "qdbus invokeShortcut"]
  },
  {
    title: "Release + Packaging",
    summary: "Release workflows produce tarballs and AppImage with managed distro publishing scaffolding.",
    group: "diagnostics",
    examples: ["docs/RELEASE.md", "docs/LINUX_PACKAGING.md", "packaging/appimage"]
  }
];
