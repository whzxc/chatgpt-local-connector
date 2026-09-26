export function validDomain(value: string): boolean {
  const domain=value.trim();
  return domain.length<=253 && domain.includes('.') && domain.split('.').every(label => /^[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?$/.test(label));
}
