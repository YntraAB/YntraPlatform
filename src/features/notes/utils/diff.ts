export interface DiffSegment {
  type: 'added' | 'removed' | 'unchanged'
  text: string
}

export function getDiffSegments(oldStr: string, newStr: string): DiffSegment[] {
  const oldWords = oldStr.split(/(\s+)/)
  const newWords = newStr.split(/(\s+)/)

  const matrix = Array(oldWords.length + 1)
    .fill(null)
    .map(() => Array(newWords.length + 1).fill(0))

  for (let i = 1; i <= oldWords.length; i++) {
    for (let j = 1; j <= newWords.length; j++) {
      if (oldWords[i - 1] === newWords[j - 1]) {
        matrix[i][j] = matrix[i - 1][j - 1] + 1
      } else {
        matrix[i][j] = Math.max(matrix[i - 1][j], matrix[i][j - 1])
      }
    }
  }

  const segments: DiffSegment[] = []
  let i = oldWords.length,
    j = newWords.length

  while (i > 0 || j > 0) {
    if (i > 0 && j > 0 && oldWords[i - 1] === newWords[j - 1]) {
      segments.unshift({ type: 'unchanged', text: oldWords[i - 1] })
      i--
      j--
    } else if (j > 0 && (i === 0 || matrix[i][j - 1] >= matrix[i - 1][j])) {
      segments.unshift({ type: 'added', text: newWords[j - 1] })
      j--
    } else if (i > 0 && (j === 0 || matrix[i][j - 1] < matrix[i - 1][j])) {
      segments.unshift({ type: 'removed', text: oldWords[i - 1] })
      i--
    }
  }

  return segments
}
