import type { ReactNode } from "react";
import { Callout, Flex, Spinner, Theme } from "@radix-ui/themes";
import { AlertCircle } from "lucide-react";
import { Icon } from "./index";
import { dark } from "../../theme";
import "@radix-ui/themes/styles.css";
import "./Feedback.css";

export function ErrorCallout({ children, action }: { children: ReactNode; action?: ReactNode }) {
  const isDark = dark.use();
  return <Theme appearance={isDark ? "dark" : "light"} className="ui-feedback-theme">
    <Callout.Root color="red" size="2" role="alert" className="ui-error-callout">
      <Callout.Icon><Icon icon={AlertCircle} size={20} /></Callout.Icon>
      <Flex align="center" gap="3" wrap="wrap" className="ui-error-content">
        <Callout.Text className="ui-error-message">{children}</Callout.Text>
        {action && <div className="ui-error-action">{action}</div>}
      </Flex>
    </Callout.Root>
  </Theme>;
}

export function LoadingIndicator({ label }: { label: string }) {
  const isDark = dark.use();
  return <Theme appearance={isDark ? "dark" : "light"} className="ui-feedback-theme">
    <span role="status" aria-label={label}><Spinner size="3" /></span>
  </Theme>;
}
