export default function SectionHeader({
  children,
  right,
}: {
  children: React.ReactNode;
  right?: React.ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-3 mb-5">
      <div className="flex items-center gap-3">
        <div className="w-0.5 h-5 bg-orange-500 rounded-full shrink-0" />
        <h2 className="font-sans font-bold text-xs tracking-widest uppercase text-[#c9d1d9]">
          {children}
        </h2>
      </div>
      {right}
    </div>
  );
}
