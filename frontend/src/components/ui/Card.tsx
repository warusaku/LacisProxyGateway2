'use client';

interface CardProps {
  title?: string;
  children: React.ReactNode;
  className?: string;
}

export function Card({ title, children, className = '' }: CardProps) {
  return (
    <div className={`bg-card border border-border rounded-lg ${className}`}>
      {title && (
        <div className="px-4 py-3 border-b border-border">
          <h3 className="font-semibold">{title}</h3>
        </div>
      )}
      <div className="p-4">{children}</div>
    </div>
  );
}
