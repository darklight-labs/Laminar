const ZATOSHI_PER_ZEC = 100_000_000n;

export function zatoshiToZec(zatoshis: string | bigint): string {
  const value = typeof zatoshis === "bigint" ? zatoshis : parseZatoshiString(zatoshis);
  const whole = value / ZATOSHI_PER_ZEC;
  const frac = value % ZATOSHI_PER_ZEC;
  if (frac === 0n) {
    return whole.toString();
  }

  let fracString = frac.toString().padStart(8, "0");
  while (fracString.endsWith("0")) {
    fracString = fracString.slice(0, -1);
  }

  return `${whole.toString()}.${fracString}`;
}

export function zecToZatoshi(input: string): string {
  const trimmed = input.trim();
  if (!trimmed) {
    throw new Error("amount is empty");
  }

  const [wholePart, fracPart = ""] = trimmed.split(".");
  if (!/^\d+$/.test(wholePart) || (fracPart && !/^\d+$/.test(fracPart))) {
    throw new Error("invalid numeric amount");
  }
  if (fracPart.length > 8) {
    throw new Error("too many decimal places");
  }

  const paddedFrac = `${fracPart}00000000`.slice(0, 8);
  return (BigInt(wholePart) * ZATOSHI_PER_ZEC + BigInt(paddedFrac)).toString();
}

function parseZatoshiString(input: string): bigint {
  const trimmed = input.trim();
  if (!/^(0|[1-9]\d*)$/.test(trimmed)) {
    throw new Error("zatoshi value must be an unsigned integer string");
  }
  return BigInt(trimmed);
}
