import type { CSSProperties, ReactNode } from "react";
import { Table, Theme } from "@radix-ui/themes";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { Button, Icon } from "./index";
import { locale, t } from "../../i18n";
import { dark } from "../../theme";
import "@radix-ui/themes/styles.css";
import "./DataTable.css";

interface Column<T> {
  key: string;
  title?: ReactNode;
  width: number;
  align?: "right";
  render: (row: T) => ReactNode;
}

export function DataTable<T extends { id: string }>({ rows, columns, onOpen, empty }: {
  rows: T[];
  columns: Column<T>[];
  onOpen: (row: T) => void;
  empty: ReactNode;
}) {
  const isDark = dark.use();
  return (
    <Theme appearance={isDark ? "dark" : "light"} className="data-table-theme">
      <Table.Root className="data-table" size="2" layout="fixed" style={{ "--data-table-width": `${columns.reduce((sum, column) => sum + column.width, 0)}px` } as CSSProperties}>
        <Table.Header>
          <Table.Row>
            {columns.map(column => <Table.ColumnHeaderCell key={column.key} style={{ width: column.width, textAlign: column.align }}>{column.title}</Table.ColumnHeaderCell>)}
          </Table.Row>
        </Table.Header>
        <Table.Body>
          {rows.map(row => <Table.Row key={row.id} className="data-table-row" tabIndex={-1} onClick={event => { event.currentTarget.focus({ preventScroll: true }); onOpen(row); }}>
            {columns.map(column => <Table.Cell key={column.key} style={{ textAlign: column.align }}>{column.render(row)}</Table.Cell>)}
          </Table.Row>)}
          {!rows.length && <Table.Row><Table.Cell colSpan={columns.length}>{empty}</Table.Cell></Table.Row>}
        </Table.Body>
      </Table.Root>
    </Theme>
  );
}

export function Pagination({ current, total, pageSize, disabled, onChange }: {
  current: number;
  total: number;
  pageSize: number;
  disabled?: boolean;
  onChange: (page: number) => void;
}) {
  locale.use();
  const last = Math.max(1, Math.ceil(total / pageSize));
  if (last === 1) return null;
  const pages = Array.from(new Set([1, ...Array.from({ length: 5 }, (_, i) => Math.min(Math.max(current - 2, 1), Math.max(last - 4, 1)) + i).filter(page => page <= last), last])).sort((a, b) => a - b);
  return <nav className="data-pagination" aria-label={t("insights.pagination")}>
    <Button variant="ghost" disabled={disabled || current <= 1} aria-label={t("insights.previous")} onClick={() => onChange(current - 1)}><Icon icon={ChevronLeft} /></Button>
    {pages.map((page, index) => <span className="data-pagination-item" key={page}>
      {index > 0 && page - pages[index - 1] > 1 && <span className="data-pagination-gap" aria-hidden="true">…</span>}
      <Button variant="ghost" disabled={disabled} aria-current={page === current ? "page" : undefined} onClick={() => onChange(page)}>{page}</Button>
    </span>)}
    <Button variant="ghost" disabled={disabled || current >= last} aria-label={t("insights.next")} onClick={() => onChange(current + 1)}><Icon icon={ChevronRight} /></Button>
  </nav>;
}
