type AddonIconVariant = "tile" | "sidebar";

const ADDON_ICON_PATHS: Record<string, string[]> = {
  Analytics: ["M4 20V10M10 20V4M16 20v-7M3 20h18"],
  "Approval Workflows": [
    "M12 3a9 9 0 1 0 9 9",
    "m8 12 2.5 2.5L17 8",
  ],
  "Feature Flags": [
    "M6 21V4",
    "M6 5h11l-2.5 4L17 13H6",
  ],
  "Priority Support": [
    "M4 14v-2a8 8 0 0 1 16 0v2",
    "M4 14h3v6H5a1 1 0 0 1-1-1v-5ZM20 14h-3v6h2a1 1 0 0 0 1-1v-5ZM17 20c0 1-1.5 2-3 2h-2",
  ],
};

export function addonClassName(name: string): string {
  return name.toLowerCase().replace(/ /g, "-");
}

export function AddonIcon({
  name,
  variant = "tile",
}: {
  name: string;
  variant?: AddonIconVariant;
}) {
  const paths = ADDON_ICON_PATHS[name] ?? ["M12 3v18M3 12h18"];
  const icon = (
    <svg
      className={
        variant === "sidebar"
          ? "sidebar-icon addon-sidebar-icon"
          : "addon-icon"
      }
      viewBox="0 0 24 24"
      aria-hidden="true"
    >
      {paths.map((path) => (
        <path d={path} key={path} />
      ))}
    </svg>
  );

  if (variant === "sidebar") return icon;

  return (
    <span className={`application-icon ${addonClassName(name)}`}>
      {icon}
    </span>
  );
}
