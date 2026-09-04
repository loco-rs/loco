export type AddonDetails = {
  description: string;
  introduction: string;
  highlights: Array<{ title: string; description: string }>;
};

const ADDON_DETAILS: Record<string, AddonDetails> = {
  Analytics: {
    description: "Explore workspace activity and usage trends.",
    introduction:
      "Analytics brings your workspace activity into a simple overview so your team can spot momentum, workload changes, and useful trends.",
    highlights: [
      { title: "Activity snapshot", description: "Review recent workspace activity at a glance." },
      { title: "Usage trends", description: "See how engagement changes over time." },
      {
        title: "Shareable insights",
        description: "Summarize the signals that matter to your team.",
      },
    ],
  },
  "Approval Workflows": {
    description: "Route work through review and sign-off stages.",
    introduction:
      "Approval Workflows gives teams a clear path from draft to final sign-off, with ownership and review status visible at every stage.",
    highlights: [
      { title: "Review stages", description: "Organize repeatable review and approval steps." },
      {
        title: "Clear ownership",
        description: "Show who is responsible for the next decision.",
      },
      {
        title: "Decision history",
        description: "Keep a simple record of approvals and changes.",
      },
    ],
  },
  "Feature Flags": {
    description: "Control staged feature releases across environments.",
    introduction:
      "Feature Flags provides a safe control center for gradual releases, internal previews, and quick feature rollbacks.",
    highlights: [
      {
        title: "Staged releases",
        description: "Roll features out gradually to selected audiences.",
      },
      {
        title: "Environment control",
        description: "Manage development and production states separately.",
      },
      {
        title: "Fast rollback",
        description: "Turn a feature off without shipping another release.",
      },
    ],
  },
  "Priority Support": {
    description: "Get expedited help from the support team.",
    introduction:
      "Priority Support gives your workspace a faster path to product guidance and help when an important workflow is blocked.",
    highlights: [
      {
        title: "Priority queue",
        description: "Move support requests into an expedited queue.",
      },
      {
        title: "Guided resolution",
        description: "Work directly with a support specialist.",
      },
      {
        title: "Workspace context",
        description: "Keep assistance focused on your team's setup.",
      },
    ],
  },
};

const FALLBACK_DETAILS: AddonDetails = {
  description: "Extend this workspace with an optional feature.",
  introduction:
    "This add-on extends the workspace with tools available through its current subscription.",
  highlights: [
    {
      title: "Workspace ready",
      description: "Available to everyone in this subscribed workspace.",
    },
    {
      title: "Easy to use",
      description: "Designed to fit the existing workspace experience.",
    },
    {
      title: "Subscription managed",
      description: "Access follows the workspace subscription.",
    },
  ],
};

export function addonDetailsFor(name: string): AddonDetails {
  return ADDON_DETAILS[name] ?? FALLBACK_DETAILS;
}
