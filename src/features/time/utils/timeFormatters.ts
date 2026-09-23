export const formatMonthYear = (ym: string): string => {
  if (!ym || !ym.includes('-')) return ym
  try {
    const [yearStr, monthStr] = ym.split('-')
    const year = parseInt(yearStr, 10)
    const month = parseInt(monthStr, 10)
    if (isNaN(year) || isNaN(month)) return ym
    const d = new Date(year, month - 1, 1)
    const monthName = d.toLocaleDateString('sv-SE', { month: 'long' })
    const capitalized = monthName.charAt(0).toUpperCase() + monthName.slice(1)
    return `${capitalized} ${year}`
  } catch {
    return ym
  }
}
