import type { ReactNode } from "react";

export default function SectionHeader({
  children,
  right,
}: {
  children: ReactNode;
  right?: ReactNode;
}) {
  return (
    <div className="section-header flex items-center justify-between gap-3 mb-5">
      <div className="section-header__left flex items-center gap-3">
        <div className="section-header__accent w-0.5 h-5 bg-orange-500 rounded-full shrink-0" />
        <h2 className="section-header__title font-sans font-bold text-xs tracking-widest uppercase text-[#c9d1d9]">
          {children}
        </h2>
      </div>
      {right}
    </div>
  );
}
