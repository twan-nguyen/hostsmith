export { cn } from "cn"

export const errorMessage = (error: unknown) =>
  error instanceof Error ? error.message : String(error);

/** "1 entry", "2 entries": the count and the noun in the form it takes. */
export const plural = (count: number, one: string, many = `${one}s`) => `${count} ${count === 1 ? one : many}`;
