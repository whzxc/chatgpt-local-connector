import { useEffect, type ReactNode } from "react";
import { locale } from "../i18n";
import { observeNativeTheme } from "../theme";
import { TooltipProvider } from "./ui";
export default function UiProvider({ children }: { children: ReactNode }) {
  const language = locale.use();
  useEffect(observeNativeTheme, []);
  return (
    <TooltipProvider delayDuration={350}>
      <div
        className="ui-root"
        lang={language}
        onClickCapture={(event) => {
          // WebKit does not focus buttons on pointer or accessibility activation. Keep dialog
          // return focus consistent with keyboard activation on every host.
          if (event.button !== 0 || !(event.target instanceof Element)) return;
          const control = event.target.closest<HTMLElement>(
            'button, a[href], [role="button"], [role="radio"]',
          );
          if (control && !control.matches('[disabled], [aria-disabled="true"]'))
            control.focus({ preventScroll: true });
        }}
      >
        {children}
      </div>
    </TooltipProvider>
  );
}
