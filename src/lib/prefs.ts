/**
 * Shared helpers for persisted enum-valued preferences (theme, surface,
 * density, …). Kept DOM-free so they are unit-testable without a backend.
 */

/** Validate an unknown stored value against an allow-list, else fall back. */
export function resolveEnum<T extends string>(
  allowed: readonly T[],
  value: unknown,
  fallback: T,
): T {
  return typeof value === "string" && (allowed as readonly string[]).includes(value)
    ? (value as T)
    : fallback;
}
