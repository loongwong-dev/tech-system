import { lazy } from "react";

export const LessonPage = lazy(() =>
  import("../pages/LessonPage").then((module) => ({ default: module.LessonPage })),
);
export const ReviewPage = lazy(() =>
  import("../pages/ReviewPage").then((module) => ({ default: module.ReviewPage })),
);
export const StudentsPage = lazy(() =>
  import("../pages/StudentsPage").then((module) => ({ default: module.StudentsPage })),
);
export const SettingsPage = lazy(() =>
  import("../pages/SettingsPage").then((module) => ({ default: module.SettingsPage })),
);
