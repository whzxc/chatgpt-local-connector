// Names and descriptions belong to the enclosing control, not a second SVG tooltip.
export const decorativeSvg = (svg: string) => svg.replace(/<title\b[^>]*>[\s\S]*?<\/title>/gi, '');
