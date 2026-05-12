import { format } from "date-fns";
import { CalendarIcon, XIcon } from "lucide-react";
import * as React from "react";

import { Button } from "@/components/ui/button";
import { Calendar } from "@/components/ui/calendar";
import { Field, FieldLabel } from "@/components/ui/field";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";

type BasicDatePickerProps = {
  id: string;
  label: string;
  value: string;
  placeholder?: string;
  onChange: (value: string) => void;
};

export function BasicDatePicker({
  id,
  label,
  value,
  placeholder = "选择日期",
  onChange,
}: BasicDatePickerProps) {
  const [open, setOpen] = React.useState(false);
  const selected = parseDate(value);

  function selectDate(date: Date | undefined) {
    if (!date) {
      return;
    }

    onChange(toRfc3339Date(date, value));
    setOpen(false);
  }

  return (
    <Field>
      <FieldLabel htmlFor={id}>{label}</FieldLabel>
      <div className="flex gap-2">
        <Popover open={open} onOpenChange={setOpen}>
          <PopoverTrigger asChild>
            <Button
              id={id}
              type="button"
              variant="outline"
              className="min-w-0 flex-1 justify-start font-normal"
            >
              <CalendarIcon data-icon="inline-start" />
              {selected ? format(selected, "yyyy-MM-dd") : placeholder}
            </Button>
          </PopoverTrigger>
          <PopoverContent className="w-auto overflow-hidden p-0" align="start">
            <Calendar
              mode="single"
              selected={selected}
              defaultMonth={selected}
              captionLayout="dropdown"
              onSelect={selectDate}
            />
          </PopoverContent>
        </Popover>
        <Button
          type="button"
          variant="outline"
          size="icon"
          disabled={!value}
          onClick={() => onChange("")}
        >
          <XIcon />
          <span className="sr-only">清空日期</span>
        </Button>
      </div>
    </Field>
  );
}

function parseDate(value: string) {
  if (!value.trim()) {
    return undefined;
  }

  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? undefined : date;
}

function toRfc3339Date(date: Date, previousValue: string) {
  const previous = parseDate(previousValue);
  const next = new Date(date);

  if (previous) {
    next.setHours(
      previous.getHours(),
      previous.getMinutes(),
      previous.getSeconds(),
      previous.getMilliseconds(),
    );
  } else {
    next.setHours(0, 0, 0, 0);
  }

  return next.toISOString();
}
