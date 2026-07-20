import { ArrowRight } from "lucide-react";
import { Link } from "react-router";

type GameCardProps = {
  title: string;
  subtitle: string;
  image: string;
  href: string;
};

export function GameCard({ title, subtitle, image, href }: GameCardProps) {
  return (
    <Link
      to={href}
      className="group relative block aspect-[16/9] overflow-hidden rounded-lg border bg-card shadow-sm transition-shadow hover:shadow-md"
    >
      <img
        src={image}
        alt=""
        className="absolute inset-0 h-full w-full object-cover transition-transform duration-500 group-hover:scale-105"
      />
      <div className="absolute inset-0 bg-gradient-to-t from-black/75 via-black/25 to-black/5" />

      <div className="absolute inset-x-0 bottom-0 p-4 text-white">
        <div className="flex min-w-0 items-center gap-3">
          <h2 className="shrink-0 text-2xl font-semibold leading-none">
            {title}
          </h2>
          <p className="ml-auto min-w-0 truncate text-right text-sm opacity-0 transition-opacity duration-200 group-hover:opacity-85">
            {subtitle}
          </p>
          <ArrowRight className="size-5 shrink-0 translate-x-1 opacity-0 transition-all duration-200 group-hover:translate-x-0 group-hover:opacity-90" />
        </div>
      </div>
    </Link>
  );
}
