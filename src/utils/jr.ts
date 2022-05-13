const JR_LINE_MAX_ID = 6;

export const isJRLine = (companyId: number): boolean =>
  companyId <= JR_LINE_MAX_ID;
