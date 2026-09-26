import { decorativeSvg } from "../decorative";
import { createElement, type FC } from "react";
import { Globe } from "lucide-react";
import ngrok from "./ngrok.png";
import pinggy from "./pinggy.png";
import localxpose from "./localxpose.png";
import cloudflare from "./cloudflare.svg?raw";

const CloudflareIcon: FC = () =>
  createElement("svg", {
    viewBox: "0 0 24 24",
    fill: "currentColor",
    dangerouslySetInnerHTML: {
      __html: decorativeSvg(cloudflare).replace(/^<svg[^>]*>|<\/svg>$/g, ""),
    },
  });

export const providerIcons = {
  ngrok: () => createElement("img", { src: ngrok, alt: "" }),
  cloudflare: CloudflareIcon,
  pinggy: () => createElement("img", { src: pinggy, alt: "" }),
  localxpose: () => createElement("img", { src: localxpose, alt: "" }),
  custom: Globe,
};
