import { useState } from "react";
import { Dialog, SingleChoice } from "../components/ui";
import { Expandable } from "./components";
import { text } from "./format";

export type GuideSection = "start" | "boundaries" | "recovery";
const topics = {
  start: ["open", "empty", "binding", "quota", "agents"],
  boundaries: ["entrypoints", "data", "lifecycle", "permissions"],
  recovery: ["connection", "version", "update", "remove"],
} as const;

export function Guide({ section, close }: { section: GuideSection; close: () => void }) {
  const [tab, setTab] = useState<GuideSection>(section);
  return (
    <Dialog title={text("guide")} onClose={close} width={620}>
      <div className="insight-guide">
        <SingleChoice
          label={text("guide")}
          value={tab}
          onChange={(value) => setTab(value as GuideSection)}
          options={(["start", "boundaries", "recovery"] as const).map((value) => ({
            value, label: text(`guide.${value}`),
          }))}
        />
        <div key={tab}>
          {topics[tab].map((topic) => (
            <Expandable key={topic} title={<span>{text(`guide.${topic}.title`)}</span>}>
              <p>{text(`guide.${topic}.body`)}</p>
            </Expandable>
          ))}
        </div>
      </div>
    </Dialog>
  );
}
