import { Page } from "../components/page";
import { ClassSection } from "../features/students/ClassSection";
import { StudentSection } from "../features/students/StudentSection";

export function StudentsPage() {
  return (
    <Page title="学员与班级">
      <div className="grid gap-6 lg:grid-cols-2">
        <StudentSection />
        <ClassSection />
      </div>
    </Page>
  );
}
