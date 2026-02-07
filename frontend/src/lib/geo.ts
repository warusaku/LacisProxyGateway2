/** Convert ISO 3166-1 alpha-2 country code to flag emoji */
export function countryCodeToFlag(code: string): string {
  const upper = code.toUpperCase();
  const codePoints: number[] = [];
  for (let i = 0; i < upper.length; i++) {
    codePoints.push(0x1f1e6 + upper.charCodeAt(i) - 65);
  }
  return String.fromCodePoint(...codePoints);
}
