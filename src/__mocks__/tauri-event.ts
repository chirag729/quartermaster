export async function listen(_event: string, _handler: (event: unknown) => void): Promise<() => void> {
  return () => {};
}

export async function emit(_event: string, _payload?: unknown): Promise<void> {}
