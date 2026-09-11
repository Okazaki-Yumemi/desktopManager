/// Command palette visibility (Ctrl+K). The actions themselves live in the
/// component so they stay close to their icons/labels.
export const palette = $state({ open: false });

export function setPaletteOpen(open: boolean): void {
  palette.open = open;
}

export function togglePalette(): void {
  palette.open = !palette.open;
}
