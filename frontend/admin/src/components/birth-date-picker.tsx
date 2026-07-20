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

type BirthDatePickerProps = {
  id: string;
  label: string;
  month: string;
  day: string;
  onChange: (value: { month: string; day: string }) => void;
};

export function BirthDatePicker({
  id,
  label,
  month,
  day,
  onChange,
}: BirthDatePickerProps) {
  const [open, setOpen] = React.useState(false);
  const selected = parseBirthDate(month, day);

  function selectDate(date: Date | undefined) {
    if (!date) {
      return;
    }

    onChange({
      month: String(date.getMonth() + 1),
      day: String(date.getDate()),
    });
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
              {selected ? format(selected, "MM-dd") : "选择生日"}
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
          disabled={!month && !day}
          onClick={() => onChange({ month: "", day: "" })}
        >
          <XIcon />
          <span className="sr-only">清空生日</span>
        </Button>
      </div>
    </Field>
  );
}

function parseBirthDate(month: string, day: string) {
  const monthValue = Number(month);
  const dayValue = Number(day);

  if (!monthValue || !dayValue) {
    return undefined;
  }

  const date = new Date(2000, monthValue - 1, dayValue);
  return Number.isNaN(date.getTime()) ? undefined : date;
}
