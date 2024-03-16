/* eslint-disable @typescript-eslint/no-explicit-any */
export const load: any = ({ params }: { params: { team: string } }) => {
	return {
		team: params.team
	};
};

export interface TeamPage {
	team: string;
}
